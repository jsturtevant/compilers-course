### libfl.so: undefined reference to 'yylex'
- add `%option noyywrap` in the cool.flex
- Remove the `-lfl` from `LIB=` in the makefile