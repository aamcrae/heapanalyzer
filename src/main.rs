use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

// JSON 'meta' object, which holds the field mappings
// for field and type usage in nodes and edges.
#[derive(Serialize, Deserialize, Debug)]
struct Meta {
    node_fields: Vec<String>,
    node_types: Value,
    edge_fields: Vec<String>,
    edge_types: Value,
    #[serde(default)]
    trace_function_info_fields: Vec<String>,
    #[serde(default)]
    trace_node_fields: Vec<String>,
    #[serde(default)]
    sample_fields: Vec<String>,
    #[serde(default)]
    location_fields: Vec<String>,
}

// JSON 'snapshot' holding the metadata and counts
// of the heap snapshot.
#[derive(Serialize, Deserialize, Debug)]
struct Snapshot {
    meta: Meta,
    node_count: usize,
    edge_count: usize,
    #[serde(default)]
    trace_function_count: usize,
}

// Top level JSON object containing all of the
// heap snapshot information.
#[derive(Serialize, Deserialize, Debug)]
struct Heap {
    snapshot: Snapshot,
    edges: Vec<i32>,
    nodes: Vec<i32>,
    #[serde(default)]
    locations: Vec<i32>,
    strings: Vec<String>,
}

// Metadata indices.
struct Index {
    // Node fields
    n_type: usize,
    n_name: usize,
    n_id: usize,
    n_size: usize,
    n_edge_count: usize,

    // Node types
    nt_obj: usize,

    // Edge fields
    e_type: usize,
    e_name: usize,
    e_node: usize,

    // Edge types
    et_property: i32,
    et_element: i32,

    // Flattened type arrays
    _node_types: Vec<String>,
    edge_types: Vec<String>,
}

// Map entry for holding object counts and sizes.
// Note the size is the 'self_size', which does not
// include referenced objects.
struct Entry {
    count: u32,
    size: i32,
    extsize: i32,
}

#[derive(PartialEq)]
struct Node {
    name: i32,
    size: i32,
    ntype: usize,
    index: usize,
    edges: (usize, usize),
}

#[derive(PartialEq)]
struct Edge {
    etype: i32,
    id: i32,
    name: i32,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let file_name: String;
    if args.len() == 2 {
        file_name = args[1].to_owned();
    } else {
        file_name = String::from("/tmp/heap.snapshot");
    }
    let h = read_snapshot_from_file(file_name).unwrap();
    println!(
        "Node count: {}, edge count: {}, string count: {}",
        h.snapshot.node_count,
        h.snapshot.edge_count,
        h.strings.len()
    );
    let meta = Index::new(&h);
    // node_len is the size of each node's data. The node data is a
    // single array of flattened values.
    let node_len = h.snapshot.meta.node_fields.len();

    // edge_len is the size of each edge entry's data. The edge data is a
    // single array of flattened values.
    let edge_len = h.snapshot.meta.edge_fields.len();

    let mut node_map = HashMap::new();
    let mut edge_list = Vec::new();
    let mut edge_index = 0;
    // Build a map of all the nodes.
    for i in (0..(h.nodes.len() - node_len - 1)).step_by(node_len) {
        let node = &h.nodes[i..(i + node_len)];
        if node_map.contains_key(&node[meta.n_id]) {
            println!("Node {} duplicate!", node[meta.n_id]);
            continue;
        }
        // Process the edges for this node.
        let slice_start = edge_list.len();
        for _ in 0..node[meta.n_edge_count] {
            // Take a slice of the edge array to bound the range.
            let edge_sl = &h.edges[edge_index..(edge_index + edge_len)];
            edge_list.push(Edge {
                etype: edge_sl[meta.e_type],
                id: edge_sl[meta.e_node],
                name: edge_sl[meta.e_name],
            });
            edge_index += edge_len;
        }
        let nentry = Node {
            name: node[meta.n_name],
            size: node[meta.n_size],
            ntype: node[meta.n_type] as usize,
            index: i / node_len,
            edges: (slice_start, slice_start + meta.n_edge_count + 1),
        };
        node_map.insert(node[meta.n_id], nentry);
    }

    // Iterate through the nodes, and build a map of the objects.
    let mut ec = vec![0; meta.edge_types.len()];
    let mut obj_map = HashMap::new();
    for (_nk, ne) in &node_map {
        if ne.ntype == meta.nt_obj {
            let e = obj_map.entry(&h.strings[ne.name as usize]).or_insert(Entry {
                count: 0,
                size: 0,
                extsize: 0,
            });
            e.count += 1;
            e.size += ne.size;
            e.extsize += ne.size;
            // Process the edges for this node.
            for i in ne.edges.0..ne.edges.1 {
                e.extsize += walk_edge(&meta, &edge_list, &node_map, i);
                let edge = &edge_list[i];
                ec[edge.etype as usize] += 1;
            }
        }
    }

    for i in 0..ec.len() {
        print!("{}: {}", meta.edge_types[i], ec[i]);
        if i != (ec.len() - 1) {
            print!(", ");
        } else {
            println!();
        }
    }

    // Print the top 30.
    top("Objects", 30, obj_map);
}

// Read a heap snapshot from a file.
fn read_snapshot_from_file<P: AsRef<Path>>(path: P) -> Result<Heap, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of Heap.
    let h: Heap = serde_json::from_reader(reader)?;

    // Return the snapshot
    Ok(h)
}

// Walk an edge and return the accumulated size.
fn walk_edge(m: &Index, el: &[Edge], nm: &HashMap<i32, Node>, index: usize) -> i32 {
    let edge = &el[index];
    if edge.etype == m.et_property || edge.etype == m.et_element {
        match nm.get(&(edge.id / 3 + 1)) {
            Some(pn) => {
                return pn.size;
            }
            None => {
                println!("Missing node, id {}, from edge {}", edge.id / 3 + 1, index);
            }
        }
    }
    0
}

// Flatten the JSON structure into a string vector.
// The JSON is structured as [ [ string,... ], string,...]
fn flatten_type(v: &Value) -> Vec<String> {
    let mut fa = Vec::new();
    flatten_next(v, &mut fa);
    fa
}

// Recursive function to flatten the JSON
fn flatten_next(v: &Value, flattened: &mut Vec<String>) {
    match v {
        Value::Array(av) => {
            // Iterate through this array.
            for v in av {
                flatten_next(&v, flattened);
            }
        }
        Value::String(nm) => {
            flattened.push(String::from(nm));
        }
        _ => (),
    }
}

// Summarise this map and display the top N count and size entries.
fn top(name: &str, mut count: usize, m: HashMap<&String, Entry>) {
    if count >= m.len() {
        count = m.len();
    }
    println!("Top {} {} by count:", count, name);
    println!("=====================");

    // Convert map to Vec of (key, value) pairs.
    let mut v = Vec::from_iter(m.iter());

    // Sort vector by count
    v.sort_by(|a, b| b.1.count.cmp(&a.1.count));

    // Only interested in the top counts.
    for ent in &v[0..count] {
        println!("{:<30} {:<12}", ent.0, ent.1.count);
    }
    println!("");

    // Now do the same for the size.
    v.sort_by(|a, b| b.1.size.cmp(&a.1.size));
    println!("Top {} {} by size (and extended size)", count, name);
    println!("=====================");
    for ent in &v[0..count] {
        println!("{:<30} {:<12} {:<12}", ent.0, ent.1.size, ent.1.extsize);
    }
}

// Constructor for metadata holder
impl Index {
    fn new(h: &Heap) -> Index {
        let node_types = flatten_type(&h.snapshot.meta.node_types);
        let edge_types = flatten_type(&h.snapshot.meta.edge_types);
        Index {
            n_type: h
                .snapshot
                .meta
                .node_fields
                .iter()
                .position(|s| s == "type")
                .unwrap(),
            n_name: h
                .snapshot
                .meta
                .node_fields
                .iter()
                .position(|s| s == "name")
                .unwrap(),
            n_id: h
                .snapshot
                .meta
                .node_fields
                .iter()
                .position(|s| s == "id")
                .unwrap(),
            n_size: h
                .snapshot
                .meta
                .node_fields
                .iter()
                .position(|s| s == "self_size")
                .unwrap(),
            n_edge_count: h
                .snapshot
                .meta
                .node_fields
                .iter()
                .position(|s| s == "edge_count")
                .unwrap(),
            nt_obj: node_types.iter().position(|s| s == "object").unwrap(),
            e_type: h
                .snapshot
                .meta
                .edge_fields
                .iter()
                .position(|s| s == "type")
                .unwrap(),
            e_name: h
                .snapshot
                .meta
                .edge_fields
                .iter()
                .position(|s| s == "name_or_index")
                .unwrap(),
            e_node: h
                .snapshot
                .meta
                .edge_fields
                .iter()
                .position(|s| s == "to_node")
                .unwrap(),
            et_property: edge_types.iter().position(|s| s == "property").unwrap() as i32,
            et_element: edge_types.iter().position(|s| s == "element").unwrap() as i32,
            _node_types: node_types,
            edge_types: edge_types,
        }
    }
}
