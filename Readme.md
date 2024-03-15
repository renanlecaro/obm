# Our beautiful mess (obm)

This is a markdown to markdown converter. That sounds a bit silly, I know.
All it does is look for lines that have an arrow in them, and consider them
as an instruction, like below :

    A->B

What we just did is declare a link from A to B. OBM will replace it by a small
chart, rendered in text in a Markdown code block (indented with 4 blocks).

The result would look something like this (head to https://obm.lecaro.me/ to
see an interactive side by side view)

    ┏━━━┓    ┏━━━┓
    ┃ A ┃━━━━┃ B ┃
    ┗━━━┛    ┗━━━┛

You can then add a second link to your document, and it will be added to the
same chart as before.

    B->C

You can declare more than one link per line too, by chaining arrows.

    C->D->E->A

If you want, you can cluster nodes by using the colon symbol between a parent
and child, like this:

    Parent:First child -> Parent:Other child

## How does it work ?

Since the beginning, we've been adding nodes and links to the same graph, it's
just that the resulting nodes are introduced one by one. The nodes that have
already been introduced are kept.

The rendering of the graphs is powered by a small engine I wrote in rust, and
running in a background thread in WASM. It uses a variety of methods to try to
come up with a nice and compact text representation of the chart.

It doesn't handle huge charts very well (30+ nodes) and runs single threaded in
WASM mode. I also have a compiled binary for linux that makes use of all the
threads available to generate large graphs faster.

## Roadmap

- rendering : show arrow heads, keep links direction information
- rendering : grow names rectangles to fit grid
- mutation : isolate a subset of the graph linked together, optimize it, then bring it back in
- mutation : isolate parent and its children, optimize it, then bring it back in
- mutation : move node furthest from center closer
- mutation : draw two rectangles, then swap location of all touching nodes
- mutation : use layout algorithm as mutation
- mutation :  see if there are interesting ideas in https://www.graphviz.org/docs/layouts/
- scoring : make links to self or parent look nicer
- scoring : add symmetry rules (tricky to do well)
- syntax: only consider lines indented with 4 blocks at least
- syntax: fix a node to a specific location
- syntax: split a document into chunks and start a new chart for each group
- cli use : improve one world per thread, then merge and do selection/birth every N seconds
- cli use : progress bar and intermediate results rendering to stdout
- cli use : editor mode lets you modify the graph and fix the nodes location
- wasm : allow multithreaded rendering
- output : generate svg instead of text diagram
- output : generate slides with presenter notes
 
 
 
