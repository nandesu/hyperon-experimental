use ::safer_ffi::prelude::*;

use hyperon::space::grounding::*;
use hyperon::atom::*;
use hyperon::space::Space;
use hyperon::common::shared::Shared;

use crate::atom::{atom, bindings_set, atoms_callback_t};

use std::os::raw::*;

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// grounding_space_t Functions & Struct
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

#[ffi_export]
#[derive_ReprC]
#[ReprC::opaque]
pub struct grounding_space {
    pub(crate) space: Shared<GroundingSpace>,
}

/// Returns a new grounding_space_t
/// 
/// The returned object must be freed with grounding_space_free()
#[ffi_export]
pub fn grounding_space_new() -> repr_c::Box<grounding_space> {
    Box::new(grounding_space{space: Shared::new(GroundingSpace::new())}).into()
}

/// Frees a grounding_space_t
#[ffi_export]
pub fn grounding_space_free(space: repr_c::Box<grounding_space>) {
    drop(space)
}

/// Returns `true` if two spaces are equal.  <TODO: stub documentation>
#[ffi_export]
pub fn grounding_space_eq(a: &grounding_space, b: &grounding_space) -> bool {
    a.space == b.space
}

/// Adds the supplied atom to the supplied space
/// 
/// This function takes ownership of the supplied atom, so it should not subsequently be freed
#[ffi_export]
pub fn grounding_space_add(space: &mut grounding_space, atom: repr_c::Box<atom>) {
    space.space.borrow_mut().add(atom.into().atom)
}

/// Removes the supplied atom from the space
#[ffi_export]
pub fn grounding_space_remove(space: &mut grounding_space, atom: &atom) -> bool {
    space.space.borrow_mut().remove(&atom.atom)
}

/// Replaces the `from` atom in the space with the `to` atom
/// 
/// This function takes ownership of the `to` atom, so it should not subsequently be freed
/// However, this function does not take ownership of the `from` atom
#[ffi_export]
pub fn grounding_space_replace(space: &mut grounding_space, from: &atom, to: repr_c::Box<atom>) -> bool {
    space.space.borrow_mut().replace(&from.atom, to.into().atom)
}

/// Returns the number of atoms in the supplied space
#[ffi_export]
pub fn grounding_space_len(space: &grounding_space) -> usize{
    space.space.borrow().iter().count()
}

/// Returns a pointer to an atom at a specified index in the space
/// 
/// The returned atom pointer should NOT be freed.
/// The returned atom pointer should NOT be accessed after the space has been freed
#[ffi_export]
pub fn grounding_space_get(space: &grounding_space, idx: usize) -> *const atom {
    // TODO: highly ineffective implementation, should be reworked after replacing
    // the GroundingSpace struct by Space trait in code.
    let space = space.space.borrow();
    let atom = space.iter().skip(idx).next()
        .expect(format!("Index is out of bounds: {}", idx).as_str());
    (atom as *const Atom).cast()
}

/// Performs the `pattern` query within the space, and returns a bindings_set_t representing
/// the query results.
/// 
/// The returned object must be freed with bindings_set_free()
#[ffi_export]
pub fn grounding_space_query(space: &grounding_space, pattern: &atom) -> repr_c::Box<bindings_set> {
    Box::new(bindings_set{set: space.space.query(&pattern.atom)}).into()
}

/// Performs a substitution within the space <TODO: stub documentation>
#[ffi_export]
pub fn grounding_space_subst(space: &grounding_space, pattern: &atom, templ: &atom, callback: atoms_callback_t, context: *mut c_void) {
    let result_atoms = space.space.subst(&pattern.atom, &templ.atom);
    for atom in result_atoms {
        callback((&atom as *const Atom).cast(), context);
    }
}
