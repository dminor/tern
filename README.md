![tern logo](tern-logo-small.png)

Tern
----
Tern is a logic programming language.

It is the fourth in a series of little languages I've worked on. The core ideas that I'm interested
in exploring in Tern are logic programming, generators/streams and how pattern matching can work
in a dynamically typed language.

Reserved Keywords
-----------------
The following keywords are reserved: `conj`, `disj`, `let`, `rel`, and `var`.

Syntax
------
    statement  -> comment | letbinding | expression
    comment    -> "#" .* "\n"
    letbinding -> "let" variable "=" expression
    expression -> table | relation | fncall | bindingref | goal | term
    table      -> "{" (term ":" term "," )* "}"
    relation   -> "rel" relname varlist "{" goal "}"
    fncall     -> variable "(" ((expression ",")* expression)? ")"
    relcall    -> relname "(" ((term ",")* term)? ")"
    bindingref -> variable
    goal       -> disj | conj | var | equals
    disj       -> "disj" "{" (goal "|")* goal "}"
    conj       -> "conj" "{" (goal ",")* goal "}"
    var        -> var varlist "{" goal "}"
    equals     -> term "==" term
    term       -> atom | variable
    atom       -> "'"[A-Za-z0-9]+
    varlist    -> "(" (variable ",")* variable ")"
    variable   -> [a-z][A-Za-z0-9]*
    relname    -> [A-Z][A-Za-z0-9]*

Annotated Bibliography
----------------------
**Daniel P. Friedman, William E. Byrd, Oleg Kiselyov and Jason Hemann. 2018. The Reasoned Schemer (Second Edition), The MIT Press, Cambridge, MA.**
This book is a great resource for learning about relational programming. It goes step by step from unification of
variables to the implementation of miniKanren. The two chapters on bits and bit operations are not very
inspiring, I would have preferred an interpreter and the generation of quines, and sometimes the question and
answer format of the book hides more than it reveals, but in general, it's excellent. The implementation of miniKanren
from a few primitive operations on streams is very elegant and is how the core operations in `src/logic.rs`
are implemented.

**Peter Norvig. 1991. Paradigms of Artificial Intelligence Programming, Morgan Kaufmann, San Francisco, CA.**
Chapter 11 is about logic programming. The description of unification and the accompanying test cases are particularly useful.

**Leon Stirling and Ehud Shapiro. The Art of Prolog (Second Edition), The MIT Press, Cambridge, MA.**
The first section of this book is provides a great overview of logic programming, unification, and the computational model
for logic programming.

Colophon
--------
The tern logo was made by a souless ai.