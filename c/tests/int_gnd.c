
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "int_gnd.h"

atom_t* int_type(const void* _self);
bool int_eq(const void* _a, const void* _b);
void* int_clone(const void* _self);
size_t int_display(const void* self, int8_t* buffer, size_t max_size);
void int_free(void* _self);

gnd_api_t const INT_GND_API = { 
    &int_type, 
    NULL, 
    NULL, 
    &int_eq, 
    &int_clone, 
    &int_display, 
    &int_free
};

atom_t* int_atom_new(int n) {
    int_gnd_payload_t* self = malloc(sizeof(int_gnd_payload_t));
    self->n = n;
    return atom_gnd(&INT_GND_API, self);
}

atom_t* int_atom_from_str(int8_t const* str, void* context) {
    int i;
    sscanf((char*)str, "%u", &i);
    return int_atom_new(i);
}

atom_t* int_type(const void* _self) {
    return atom_sym("int");
}

bool int_eq(const void* _a, const void* _b) {
    int_gnd_payload_t *a = (int_gnd_payload_t*)_a;
    int_gnd_payload_t *b = (int_gnd_payload_t*)_b;
    return a->n == b->n;
}

void* int_clone(const void* _self) {
    int_gnd_payload_t* copy = malloc(sizeof(int_gnd_payload_t));
    memcpy(copy, _self, sizeof(int_gnd_payload_t));
    return copy;
}

size_t int_display(const void* _self, int8_t* buffer, size_t max_size) {
    int_gnd_payload_t *self = (int_gnd_payload_t*)_self;
    return snprintf((char*)buffer, max_size, "%d", self->n);
}

void int_free(void* _self) {
    free(_self);
}
