#include <stdarg.h>
#include <string.h>

#include "util.h"

#define MAXARGS 64

atom_t* expr(atom_t* atom, ...) {
    va_list ap;
    atom_t* children[MAXARGS];
    int argno = 0;

    va_start(ap, atom);
    while (atom != 0 && argno < MAXARGS) {
        children[argno++] = atom;
        atom = va_arg(ap, atom_t*);
    }
    va_end(ap);
    return atom_expr(children, argno);
}
