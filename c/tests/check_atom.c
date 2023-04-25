
#include <stdio.h>
#include <stdlib.h>

#include <hyperon/hyperon.h>

#include "test.h"
#include "util.h"
#include "int_gnd.h"

void setup(void) {
}

void teardown(void) {
}

void print_bindings(const bindings_t* bindings, void *context) {
    char *bindings_string = bindings_to_str(bindings);
    //printf("%s\n", bindings_string);
    hyp_string_free(bindings_string);
};

START_TEST (test_bindings_set)
{
    bindings_t* bindings_a = bindings_new();
    bindings_add_var_binding(bindings_a, atom_var("a"), atom_sym("A"));

    bindings_t* bindings_b = bindings_new();
    bindings_add_var_binding(bindings_b, atom_var("b"), atom_sym("B"));

    bindings_set_t* set_1 = bindings_merge(bindings_a, bindings_b);

    bindings_t* bindings_c = bindings_new();
    bindings_add_var_binding(bindings_c, atom_var("c"), atom_sym("C"));

    bindings_set_t* set_2 = bindings_set_from_bindings(bindings_c);
    bindings_set_t* result_set = bindings_set_merge(set_1, set_2);

    bindings_set_add_var_equality(result_set, atom_var("a"), atom_var("a_prime"));

    bindings_set_add_var_binding(result_set, atom_var("d"), atom_sym("D"));

    bindings_set_iterate(result_set, &print_bindings, NULL);

    bindings_free(bindings_a);
    bindings_free(bindings_b);
    bindings_free(bindings_c);
    bindings_set_free(set_1);
    bindings_set_free(set_2);
    bindings_set_free(result_set);
}
END_TEST

START_TEST (test_sym)
{
    char name[] = "test";
    atom_t* atom = atom_sym(name);
    name[0] = 'r';
    
    char* actual = atom_to_str(atom);
    ck_assert_str_eq(actual, "test");

    hyp_string_free(actual);
    atom_free(atom);
}
END_TEST

START_TEST (test_expr)
{
    atom_t* atom = expr(atom_sym("test"), atom_var("var"), atom_sym("five"), int_atom_new(42), 0);

    char* actual = atom_to_str(atom);
    ck_assert_str_eq(actual, "(test $var five 42)");

    hyp_string_free(actual);
    atom_free(atom);
}
END_TEST

void init_test(TCase* test_case) {
    tcase_set_timeout(test_case, 300); //300s = 5min.  To test for memory leaks
    tcase_add_checked_fixture(test_case, setup, teardown);
    tcase_add_test(test_case, test_bindings_set);
    tcase_add_test(test_case, test_sym);
    tcase_add_test(test_case, test_expr);
}

TEST_MAIN(init_test);
