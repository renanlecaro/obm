# Our beautiful mess (obm)

This is a markdown to markdown converter. That sounds a bit silly, I know.
All it does is look for lines that have an arrow in them, and consider them
as an instruction, like below :

    A->B

What we just did is declare a link from A to B. OBM will replace it by a small
chart, rendered in text in a Markdown code block (indented with 4 blocks).

The result would look something like this.

    ╔═══╗ ╔═══╗
    ║ A ║═║ B ║
    ╚═══╝ ╚═══╝

Head to https://obm.lecaro.me/ to see an interactive side by side view.

## How does it work ?

The rendering of the graphs is powered by a small engine I wrote in rust, and
running in a background thread in WASM. It uses a variety of methods to try to
come up with a nice and compact text representation of the chart.

It doesn't handle huge charts very well (30+ nodes) and runs single threaded in
WASM mode. I also have a compiled binary for linux that makes use of all the
threads available to generate large graphs faster.

## Why build this ?

Sequence diagrams are great to explain processes with 3-4 participants, but don't
scale well when the number of participants goes up. I once had to explain how a SAAS
queuing system for call centers works :  callers reach a call center, are
forwarded to a twilio number, twilio reaches our API, talks to the caller, then hang up.

This kind of process is hard to explain with a simple sequence diagram. That's when i
thought that animating or modifying a flowchart would make sense.

# similar tools

graph-easy is based on dots and mostly just better than this project.
It can be easily used thanks to this hosted service
https://dot-to-ascii.ggerganov.com/

Adia does text to text for sequence diagrams
https://github.com/pylover/adia

Mermaid does text to svg/png for many graph types
https://mermaid.live/edit

Dots to ascii (python)
https://github.com/ggerganov/dot-to-ascii

Visual editor for plain text diagrams
https://asciiflow.com/#/

Text to image
http://blockdiag.com/en/blockdiag/index.html

Discussions
https://stackoverflow.com/questions/3211801/graphviz-and-ascii-output

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
 
 
 
