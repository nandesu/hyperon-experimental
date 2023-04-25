use ::safer_ffi::prelude::*;

use hyperon::*;

use std::ffi::CString;

use std::os::raw::*;
use std::fmt::Display;
use std::convert::{TryInto};

use hyperon::matcher::{Bindings, BindingsSet};

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// Structs & Types exported to C
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

#[ffi_export]
#[derive_ReprC]
#[repr(u8)]
pub enum atom_type {
    Symbol,
    Variable,
    Expr,
    Grounded,
}

#[ffi_export]
#[derive_ReprC]
#[ReprC::opaque]
pub struct atom {
    pub(crate) atom: Atom,
}

/// An error / status type that may be returned from a grounded atom's execute function
/// to provide status to the interpreter
#[ffi_export]
#[derive_ReprC]
#[ReprC::opaque]
pub struct exec_error {
    err: ExecError,
}

/// Wraps a set of variable-value associations that are simultaneously valid and non-conflicting
#[ffi_export]
#[derive_ReprC]
#[ReprC::opaque]
pub struct bindings {
    pub(crate) bindings: Bindings,
}

/// Wraps a set possible bindings_t objects, each representing an alternative bindings state
#[ffi_export]
#[derive_ReprC]
#[ReprC::opaque]
pub struct bindings_set {
    pub(crate) set: BindingsSet,
}

pub(crate) type bindings_callback_t = extern "C" fn(data: *const bindings, context: *mut c_void);
pub(crate) type atom_binding_callback_t = extern "C" fn(var: *const atom, value: *const atom, context: *mut c_void);
pub(crate) type atoms_callback_t = extern "C" fn(atom: *const atom, context: *mut c_void);

/// A table of functions to define the behavior of a grounded atom
/// 
/// type_
///   Returns an atom representing the type of the grounded atom
///   \arg payload \c is the pointer to the grounded atom's payload
///
/// execute
///   Executes the grounded atom, if this field is NULL then the atom is not executable
///   \arg payload \c is the pointer to the grounded atom's payload
///   \arg args \c is the base pointer to a vector of atom_t pointers for the arguments
///   \arg arg_count \c is the number of arguments
///   \arg result_set \c is an `vec_atom` into which new result atoms may be added
///   Returns an `exec_error_t`, or `NULL` if no error occurred
/// 
/// match_
///   Matches a grounded atom with another atom
///   \arg payload \c is the pointer to the grounded atom's payload
///   \arg other \c is the other atom
///   Returns a bindings_set_t representing all matches
/// 
/// eq
///   Returns `true` if two grounded atoms are equal
///   \arg payload \c is the pointer to the grounded atom's payload
///   \arg other_payload \c is the pointer to the other grounded atom's payload
/// 
/// clone
///   Returns another payload object, used to create a clone of the grounded atom
///   \arg payload \c is the pointer to the grounded atom's payload
/// 
/// display
///   Writes a text description of the grounded atom into a buffer
///   \arg payload \c is the pointer to the grounded atom's payload
///   \arg buffer \c is the pointer to the text buffer
///   \arg buffer_size \c is the maximum size of the buffer allocated
///   Returns the number of bytes written to the buffer
/// 
/// free
///   Deallocates the payload buffer
///   \arg payload \c is the pointer to the grounded atom's payload
#[ffi_export]
#[derive_ReprC]
#[repr(C)]
//TODO: Discussion w/ Vitaly.  In some of these functions, the atom itself might be more useful than
// the payload buffer.  For example in match_ & execute.  clone & free definitely should take the
// payload buffer.  But type_, eq & display could be argued either way.
pub struct gnd_api {
    // TODO: This function needs to return multiple atoms, when the corresponding Rust API is updated
    type_: extern "C" fn(*const c_void) -> repr_c::Box<atom>,
    // TODO: replace args by C array and ret by callback
    execute: Option<extern "C" fn(*const c_void, *const *const atom, usize, *mut vec_atom) -> Option<repr_c::Box<exec_error>>>,
    match_: Option<extern "C" fn(*const c_void, *const atom) -> repr_c::Box<bindings_set>>,
    eq: extern "C" fn(*const c_void, *const c_void) -> bool,
    clone: extern "C" fn(*const c_void) -> *mut c_void,
    //TODO.  Annoyingly, *mut c_char becomes *int8_t in the c header. Not sure what to do about it.  https://users.rust-lang.org/t/safer-ffi-question-how-to-make-c-char-into-char-rather-than-int8-t/92929
    display: extern "C" fn(*const c_void, *mut c_char, usize) -> usize,
    free: extern "C" fn(*mut c_void),
}

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// exec_error_t Functions
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

/// Creates a new exec_error_t indicating a a runtime error.  This error will halt the interpreter
/// and the message will be reported
/// 
/// The result object should be freed with exec_error_free(), if it is no longer needed
#[ffi_export]
pub fn exec_error_runtime(message: char_p::Ref<'_>) -> repr_c::Box<exec_error> {
    Box::new(exec_error{err: ExecError::Runtime(message.to_string())}).into()
}

/// Creates a new exec_error_t to indicate to the interpreter that the execution results should not
/// be further reduced
/// 
/// The result object should be freed with exec_error_free(), if it is no longer needed
#[ffi_export]
pub fn exec_error_no_reduce() -> repr_c::Box<exec_error> {
    Box::new(exec_error{err: ExecError::NoReduce}).into()
}

/// Frees an exec_error_t
/// 
/// QUESTION FOR VITALY: When will the user create an exec_error_t, that they won't pass as a return
/// value from gnd_api->execute()?
#[ffi_export]
pub fn exec_error_free(err: repr_c::Box<exec_error>) {
    drop(err)
}

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// bindings_t Functions
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

/// Creates a new bindings_t
/// 
/// The result object should be freed with bindings_free()
#[ffi_export]
pub fn bindings_new() -> repr_c::Box<bindings> {
    Box::new(bindings{bindings: Bindings::new()}).into()
}

/// Frees a bindings_t
#[ffi_export]
pub fn bindings_free(bindings: repr_c::Box<bindings>) {
    drop(bindings)
}

/// Makes a clone of a bindings_t
/// 
/// The result object should be freed with bindings_free()
#[ffi_export]
pub fn bindings_clone(bindings: &bindings) -> repr_c::Box<bindings> {
    Box::new(bindings{bindings: bindings.bindings.clone()}).into()
}

/// Creates a string description of a bindings_t
/// 
/// The result string should be freed with hyp_string_free()
#[ffi_export]
pub fn bindings_to_str(bindings: &bindings) -> char_p::Box {
    CString::new(bindings.bindings.to_string()).unwrap().into()
}

/// Compares two bindings_t objects
#[ffi_export]
pub fn bindings_eq(a: &bindings, b: &bindings) -> bool {
    a.bindings == b.bindings
}

/// Iterates all bindings within a bindings_t calling the `callback` function for each
#[ffi_export]
pub fn bindings_traverse(bindings: &bindings, callback: atom_binding_callback_t, context: *mut c_void) {
    bindings.bindings.iter().for_each(|(var, atom)|  {
            let c_var_atom = atom{atom: Atom::Variable(var.clone())};
            callback(&c_var_atom, &atom{atom: atom}, context);
        }
    )
}

/// Adds a new var-value binding to a bindings_t.  Returns `true` if the binding was added sucessfully,
/// or `false` if adding the new binding would result in a conflict with existing bindings.
/// 
/// This function takes ownership of the provided `var` and `value` atoms, so should NOT be
/// subsequently freed.
/// 
/// The `var` arg must be a variable atom, but `value` may be any atom type.
#[ffi_export]
pub fn bindings_add_var_binding(bindings: &mut bindings, var: repr_c::Box<atom>, value: repr_c::Box<atom>) -> bool {
    match bindings.bindings.clone().add_var_binding_v2(TryInto::<&VariableAtom>::try_into(&var.atom).unwrap(), &value.atom) {
        Ok(new_bindings) => {
            bindings.bindings = new_bindings;
            true
        },
        Err(_) => false
    }
}

/// Returns `true` if a bindings_t contains no bindings, otherwise returns `false`
#[ffi_export]
pub fn bindings_is_empty(bindings: &bindings) -> bool {
    bindings.bindings.is_empty()
}

/// Returns an atom from a bindings_t associated with a provided variable name, or NULL if
/// no variable binding exists
/// 
/// If a non-null value is returned, the returned atom must be freed with atom_free() or
/// provided to another function that takes ownership of the atom.
/// 
//TODO: discuss if an atom_t is more convenient than a string
#[ffi_export]
pub fn bindings_resolve(bindings: &bindings, var_name: char_p::Ref<'_>) -> Option<repr_c::Box<atom>> {
    bindings.bindings.resolve(&VariableAtom::new(var_name.to_str())).map(|atom| Box::new(atom{atom: atom}).into())
}

/// Merges two bindings_t objects to create a bindings_set_t
/// 
/// The returned object must be freed with bindings_set_free()
#[ffi_export]
pub fn bindings_merge(left: &bindings, right: &bindings) -> repr_c::Box<bindings_set> {    
    let new_set = left.bindings.clone().merge_v2(&right.bindings);
    Box::new(bindings_set{set: new_set}).into()
}

/// Returns an atom from a bindings_t associated with a provided variable name, or NULL if
/// no variable binding exists.  The atom will subsequently be removed from the bindings_t.
/// 
/// If a non-null value is returned, the returned object must be freed with atom_free() or
/// provided to another function that takes ownership of the atom.
#[ffi_export]
pub fn bindings_resolve_and_remove(bindings: &mut bindings, var_name: char_p::Ref<'_>) -> Option<repr_c::Box<atom>> {    
    bindings.bindings.resolve_and_remove(&VariableAtom::new(var_name.to_str())).map(|atom| Box::new(atom{atom: atom}).into())
}

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// bindings_set_t Functions
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

/// Returns a new bindings_set_t containing no bindings
/// 
/// The returned object must be freed with bindings_set_free()
#[ffi_export]
pub fn bindings_set_empty() -> repr_c::Box<bindings_set> {    
    Box::new(bindings_set{set: BindingsSet::empty()}).into()
}

/// Returns a new bindings_set_t containing a single empty bindings
/// 
/// The returned object must be freed with bindings_set_free()
#[ffi_export]
pub fn bindings_set_single() -> repr_c::Box<bindings_set> {    
    Box::new(bindings_set{set: BindingsSet::single()}).into()
}

/// Returns a new bindings_set_t containing the bindings from the provided bindings_t
/// 
/// The returned object must be freed with bindings_set_free()
#[ffi_export]
pub fn bindings_set_from_bindings(bindings: &bindings) -> repr_c::Box<bindings_set> {    
    Box::new(bindings_set{set: BindingsSet::from(bindings.bindings.clone())}).into()
}

/// Frees a bindings_set_t
#[ffi_export]
pub fn bindings_set_free(set: repr_c::Box<bindings_set>) {
    drop(set)
}

/// Returns the number of bindings within a bindings_set_t
#[ffi_export]
pub fn bindings_set_len(set: &bindings_set) -> usize {    
    set.set.len()
}

/// Iterates all bindings_t instances within a bindings_set_t, calling the callback function for each
#[ffi_export]
pub fn bindings_set_iterate(set: &bindings_set, callback: bindings_callback_t, context: *mut c_void) {
    for bindings in set.set.iter() {
        let bindings_ptr = (bindings as *const Bindings).cast::<bindings>();
        callback(bindings_ptr, context);
    }
}

/// Asserts two variable atoms are equal in every bindings within a bindings_set_t
/// 
/// Both `a` and `b` must be variable atoms.  This function takes ownership of both atoms and
/// therefore they should not be subsequently freed.
#[ffi_export]
pub fn bindings_set_add_var_equality(set: &mut bindings_set, a: repr_c::Box<atom>, b: repr_c::Box<atom>) {
    let mut owned_set = BindingsSet::empty();
    core::mem::swap(&mut owned_set, &mut set.set);
    let mut result_set = owned_set.add_var_equality((&a.atom).try_into().unwrap(), (&b.atom).try_into().unwrap());
    core::mem::swap(&mut result_set, &mut set.set);
}

/// Asserts two variable atoms are equal in every bindings within a bindings_set_t
/// 
/// If there is a conflict with any existing bindings, those bindings will be removed from the bindings_set_t
#[ffi_export]
pub fn bindings_set_add_var_binding(set: &mut bindings_set, var: repr_c::Box<atom>, value: repr_c::Box<atom>) {
    let mut owned_set = BindingsSet::empty();
    core::mem::swap(&mut owned_set, &mut set.set);
    let mut result_set = owned_set.add_var_binding(TryInto::<&VariableAtom>::try_into(&var.atom).unwrap(), &value.atom);
    core::mem::swap(&mut result_set, &mut set.set);
}

/// Merges two bindings_set_t objects together
/// 
/// The returned object must be freed with bindings_set_free()
#[ffi_export]
pub fn bindings_set_merge(a: &bindings_set, b: &bindings_set) -> repr_c::Box<bindings_set> {
    let result_set = a.set.clone().merge(&b.set);
    Box::new(bindings_set{set: result_set}).into()
}

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// atom_t Functions
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

/// Creates a new symbol atom with the given text
/// 
/// The returned atom must be freed with atom_free() or given to a function that assumes ownership
#[ffi_export]
pub fn atom_sym(name: char_p::Ref<'_>) -> repr_c::Box<atom> {
    Box::new(atom{atom: Atom::sym(name.to_str())}).into()
}

/// Creates a new expression atom from the provided children.  
/// 
/// This function takes ownership of all children atoms, but does not the buffer containing the
/// `children` pointers must be managed by the caller
/// 
/// The returned atom must be freed with atom_free() or given to a function that assumes ownership
#[ffi_export]
pub fn atom_expr(children: *mut repr_c::Box<atom>, size: usize) -> repr_c::Box<atom> {

    // This function violates Rust conventions, hence the unsafe.  But the alternative
    // was added runtime overhead in a commonly used function, and this calling pattern is
    // intuitive to C programmers.
    // If we want to get rid of the unsafe, the obvious choice is to take a vec_atom_t
    let c_arr: &mut [repr_c::Box<atom>] = unsafe{ std::slice::from_raw_parts_mut(children, size) };
    let children: Vec<Atom> = c_arr.into_iter().map(|atom| ptr_into_atom(atom)).collect();
    Box::new(atom{atom: Atom::expr(children)}).into()
}

/// Creates a new variable atom with the given name
/// 
/// The returned atom must be freed with atom_free() or given to a function that assumes ownership
#[ffi_export]
pub fn atom_var(name: char_p::Ref<'_>) -> repr_c::Box<atom> {
    Box::new(atom{atom: Atom::var(name.to_str())}).into()
}

/// Creates a new grounded atom with the provided api and payload
/// 
/// The returned atom must be freed with atom_free() or given to a function that assumes ownership
#[ffi_export]
pub fn atom_gnd(api: &'static gnd_api, payload: *mut c_void) -> repr_c::Box<atom> {
    Box::new(atom{atom: Atom::gnd(CGrounded{api, payload})}).into()
}

/// Returns the type of an atom_t
#[ffi_export]
pub fn atom_get_type(atom: &atom) -> atom_type {
    match atom.atom {
        Atom::Symbol(_) => atom_type::Symbol,
        Atom::Variable(_) => atom_type::Variable,
        Atom::Expression(_) => atom_type::Expr,
        Atom::Grounded(_) => atom_type::Grounded,
    }
}

/// Returns a string description of an atom_t
/// 
/// The returned string must be freed with hyp_string_free()
#[ffi_export]
pub fn atom_to_str(atom: &atom) -> char_p::Box {
    CString::new(atom.atom.to_string()).unwrap().into()
}

/// Returns the name of a symbol or variable atom
/// 
/// The returned string must be freed with hyp_string_free()
#[ffi_export]
pub fn atom_get_name(atom: &atom) -> char_p::Box {
    match &atom.atom {
        Atom::Symbol(s) => CString::new(s.name()).unwrap().into(),
        Atom::Variable(v) => CString::new(v.name()).unwrap().into(),
        _ => panic!("Only Symbol and Variable atoms have a name attribute!"),
    }
}

/// Returns a pointer to the payload of a grounded atom
/// 
/// The returned payload pointer must NOT be accessed after the atom_t has been freed or
/// after ownership of the atom has been given to another function.
/// 
/// The returned payload pointer must NOT be freed.
#[ffi_export]
pub fn atom_get_payload(atom: &atom) -> *mut c_void {
    if let Atom::Grounded(g) = &atom.atom {
        match (g).as_any_ref().downcast_ref::<CGrounded>() {
            Some(g) => g.payload,
            None => panic!("Returning payload from non C grounded objects is not supported!"),
        }
    } else {
        panic!("Only Grounded atoms have a payload!");
    }
}

/// Returns an atom indicating the type of a grounded atom
/// 
/// The returned atom must be freed with atom_free() or given to another function that
/// takes ownership.
//TODO: When the Rust API changes to support multiple types, this API will need to change also
#[ffi_export]
pub fn atom_get_grounded_type(atom: &atom) -> repr_c::Box<atom> {
    if let Atom::Grounded(g) = &atom.atom {
        Box::new(atom{atom: g.type_()}).into()
    } else {
        panic!("Only Grounded atoms has grounded type attribute!");
    }
}

/// Iterates all children atoms of an expression atom, calling the `callback` function for each one
#[ffi_export]
pub fn atom_get_children(atom: &atom, callback: atoms_callback_t, context: *mut c_void) {
    if let Atom::Expression(e) = &atom.atom {
        for child in e.children() {
            callback((child as *const Atom).cast(), context);
        }
    } else {
        panic!("Only Expression atoms have children!");
    }
}

/// Frees an atom_t
#[ffi_export]
pub fn atom_free(atom: repr_c::Box<atom>) {
    drop(atom)
}

/// Clones an atom_t
/// 
/// The returned atom must be freed with atom_free(), or it must be passed to a function
/// that accepts ownership.
#[ffi_export]
pub fn atom_clone(atom: &atom) -> repr_c::Box<atom> {
    Box::new(atom{atom: atom.atom.clone()}).into()
}

/// Returns `true` if two atom_t are equal, otherwise returns `false` 
#[ffi_export]
pub fn atom_eq(a: &atom, b: &atom) -> bool {
    a.atom == b.atom
}

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// vec_atom_t Functions & Struct
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

/// A vector of atom_t
#[ffi_export]
#[derive_ReprC]
#[ReprC::opaque]
pub struct vec_atom {
    vec: Vec<Atom>,
}

/// Creates a new empty vec_atom_t
/// 
/// The returned object must be feed with vec_atom_free()
#[ffi_export]
pub fn vec_atom_new() -> repr_c::Box<vec_atom> {
    Box::new(vec_atom{vec: Vec::new()}).into()
}

/// Frees a vec_atom_t
#[ffi_export]
pub fn vec_atom_free(vec: repr_c::Box<vec_atom>) {
    drop(vec);
}

/// Pushes an atom into a vec_atom_t
/// 
/// This function takes ownership of the provided atom, so the atom should not be freed
#[ffi_export]
pub fn vec_atom_push(vec: &mut vec_atom, atom: repr_c::Box<atom>) {
    vec.vec.push(atom.into().atom)
}

/// Removes the last atom from a vec_atom_t and returns it.  Returns NULL if the vec_atom_t is empty
/// 
/// The returned atom must be freed with atom_free() or given to another function that takes
/// ownership of the atom
#[ffi_export]
pub fn vec_atom_pop(vec: &mut vec_atom) -> Option<repr_c::Box<atom>> {
    let result = vec.vec.pop();
    result.map(|atom| Box::new(atom{atom}).into())
}

/// Gets a pointer to the atom at idx within a vec_atom_t.  Returns NULL if no atom exists at idx
/// 
/// The returned pointer must NOT be freed, nor may to be passed to a function that accepts ownership
/// Thr returned pointer must NOT be accessed after the vec_atom_t has been freed
#[ffi_export]
pub fn vec_atom_get(vec: &vec_atom, idx: usize) -> *const atom {
    let result = vec.vec.get(idx);
    match result {
        Some(atom_ref) => (atom_ref as *const Atom).cast(),
        None => std::ptr::null()
    }
}

/// Returns the number of atoms in a vec_atom_t
#[ffi_export]
pub fn vec_atom_len(vec: &mut vec_atom) -> usize {
    vec.vec.len()
}

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// Internal Functions
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

/// Nasty internal function to treat a &mut as a Boxed atom and steal ownership.
/// Used to maintain more "C-like" calling semantics of atom_expr(), but the ownership pattern
/// of atom_expr() is a bit strange and it requires the only unsafe in this whole API.
fn ptr_into_atom(atom: &mut atom) -> Atom {
    unsafe{ Box::from_raw(atom) }.atom
}

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// C Grounded Atom Wrapper
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

/// Internal struct to wrap a C implementation of a GroundedAtom
//QUESTION FOR VITALY: Originally, this type was a wrapped AtomicPtr<>, but I don't understand
// the situation where multiple threads have shared access to this pointer
struct CGrounded {
    api: &'static gnd_api,
    payload: *mut c_void,
}

impl Grounded for CGrounded {
    fn type_(&self) -> Atom {
        ((self.api.type_)(self.payload).into()).atom
    }

    //QUESTION FOR VITALY: Why does the GroundedAtom::execute trait method take a &mut Vec<>, as
    // opposed to either taking ownership or the args or just taking an immutable borrow?  My guess
    // was that you might have intended the grounded atom interface to be able to mutate argument
    // atoms, but looking at the implementation of interpreter::execute_op, that appears not to be
    // the case.
    //Also, if the idea was to mutate arguments, the results Vec<> could be redundant because
    // newly created result atoms could just be added to the vector - which would be more appropriately
    // thought of as a sub-space
    fn execute(&self, args: &mut Vec<Atom>) -> Result<Vec<Atom>, ExecError> {
        match self.api.execute {
            Some(func) => {
                let mut ret = vec_atom{vec: Vec::new()};
                let arg_ptrs: Vec<*const atom> = args.iter().map(|atom| (atom as *const Atom).cast::<atom>()).collect();
                let error = func(self.payload, arg_ptrs.as_ptr(), arg_ptrs.len(), &mut ret);
                let ret_result = match error {
                    None => Ok(ret.vec),
                    Some(err) => Err(err.into().err)
                };
                log::trace!("CGrounded::execute: atom: {:?}, args: {:?}, ret: {:?}", self, args, ret_result);
                ret_result
            },
            None => execute_not_executable(self)
        }
    }

    fn match_(&self, other: &Atom) -> matcher::MatchResultIter {
        match self.api.match_ {
            Some(func) => {
                let results = func(self.payload, (other as *const Atom).cast());
                Box::new((results.into()).set.into_iter())
            },
            None => match_by_equality(self, other)
        }
    }
}

// Two grounded atoms are the same if they have the same API, and the eq function
// returns true on their payloads
impl PartialEq for CGrounded {
    fn eq(&self, other: &CGrounded) -> bool {
        self.api as *const _ == other.api as *const _ &&
        (self.api.eq)(self.payload, other.payload)
    }
}

impl Clone for CGrounded {
    fn clone(&self) -> Self {
        let new_payload = (self.api.clone)(self.payload);
        CGrounded{api: self.api, payload: new_payload}
    }
}

impl core::fmt::Debug for CGrounded {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self}")
    }
}

impl Display for CGrounded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buffer = [0u8; 4096];
        let bytes_written = (self.api.display)(self.payload, buffer.as_mut_ptr().cast(), 4096);
        let text = std::str::from_utf8(&buffer[0..bytes_written]).expect("Incorrect UTF-8 sequence");
        write!(f, "{}", text)
    }
}

impl Drop for CGrounded {
    fn drop(&mut self) {
        (self.api.free)(self.payload);
    }
}
