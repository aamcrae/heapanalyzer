# Chrome heap memory snapshot analyzer

```heapanalyzer``` will read a Chrome memory heap snapshot and
provide a breakdown of the object types, counts and memory usage.

## How to run

 1. Navigate to the web page you wish to analyze.
 2. Select the right most 3-dot menu on the Chrome page.
 3. Select 'Developer tools' under the 'More tools' item.
 4. Select the 'Memory' item on the top bar.
 5. Select the 'Take snapshot' at the bottom of the page.
 6. Download the heap snapshot displayed on the top left panel (via tha 'Save profile' item on the 3-dot menu).

This will download a heap snapshot with a name like Heap-20250501T132336.heapsnapshot

Use this as the argument to heap analyzer:

```
cargo run -r -- ~/Downloads/Heap-20250501T132336.heapsnapshot
...
Node count: 1156182, edge count: 5394173, string count: 108786
context: 77479, element: 89355, property: 520308, internal: 262702, hidden: 3801, shortcut: 0, weak: 570, string_or_number: 0, node: 0
Top 30 Objects by count:
=====================
Object                         69545       
Array                          40795       
system / Context               36420       
Tu                             11392       
K                              4114        
za                             2363        
gd                             2266        
Promise                        1987        
...
Top 30 Objects by size (and extended size)
=====================
Object                         1855844      9583464     
Tu                             1184768      2915780     
system / Context               998696       1286720     
Array                          652792       3404636     
gd                             105056       241580      
K                              65928        395476      
e                              55708        207304      
za                             47292        389448      
Promise                        40100        173948      
...
```
