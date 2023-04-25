#ifndef INT_GND_H
#define INT_GND_H

#include <hyperon/hyperon.h>

typedef struct _int_gnd_payload_t {
    int n;
} int_gnd_payload_t;

atom_t* int_atom_new(int n);
atom_t* int_atom_from_str(int8_t const* str, void* context);

#endif /* INT_GND_H */
