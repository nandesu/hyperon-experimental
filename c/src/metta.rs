use ::safer_ffi::prelude::*;

use hyperon::*;
use hyperon::metta::text::*;
use hyperon::metta::interpreter;
use hyperon::metta::interpreter::InterpretedAtom;
use hyperon::common::plan::StepResult;
use hyperon::metta::runner::Metta;
use hyperon::common::shared::Shared;

use crate::atom::{atom, atoms_callback_t};
use crate::space::{grounding_space};

use std::os::raw::*;
use std::ffi::CString;
use regex::Regex;
use std::path::PathBuf;

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// tokenizer_t Functions & Struct
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

#[ffi_export]
#[derive_ReprC]
#[ReprC::opaque]
pub struct tokenizer {
    pub(crate) tok: Shared<Tokenizer>,
}

/// Returns a new tokenizer_t
/// 
/// The returned object must be freed with tokenizer_free()
#[ffi_export]
pub fn tokenizer_new() -> repr_c::Box<tokenizer> {
    Box::new(tokenizer{tok: Shared::new(Tokenizer::new())}).into()
}

/// Frees a new tokenizer_t
#[ffi_export]
pub fn tokenizer_free(tokenizer: repr_c::Box<tokenizer>)  {
    drop(tokenizer);
}

type atom_constr_t = extern "C" fn(*const c_char, *mut c_void) -> repr_c::Box<atom>;

/// A wrapper around an object whose memory life-cycle is managed by the HyperonC implementation
/// 
/// The `free` callback function will be called when HyperonC wishes to free the object 
#[ffi_export]
#[derive_ReprC]
#[repr(C)]
pub struct droppable {
    ptr: *mut c_void,
    free: Option<extern "C" fn(ptr: *mut c_void)>,
}

impl Drop for droppable {
    fn drop(&mut self) {
        let free = (*self).free;
        if let Some(free) = free {
            free(self.ptr);
        }
    }
}

/// Registers a new token with a tokenizer_t, allowing the interpreter to recognize input
/// text and construct atoms from the text string
#[ffi_export]
pub fn tokenizer_register_token(tokenizer: &mut tokenizer, regex: char_p::Ref<'_>,
    constr: atom_constr_t, context: droppable)  {

    let regex = Regex::new(regex.to_str()).unwrap();
    tokenizer.tok.borrow_mut().register_token(regex, move |token| {        
        let c_atom = constr((&CString::new(token).unwrap()).as_ptr(), context.ptr);
        c_atom.into().atom
    });
}

/// Clones a tokenizer_t object
/// 
/// The returned object must be freed with tokenizer_free()
#[ffi_export]
pub fn tokenizer_clone(tokenizer: &tokenizer) -> repr_c::Box<tokenizer> {
    Box::new(tokenizer{tok: tokenizer.tok.clone()}).into()
}

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// sexpr_parser_t Functions & Struct
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

#[ffi_export]
#[derive_ReprC]
#[ReprC::opaque]
pub struct sexpr_parser {
    pub(crate) parser: Shared<SExprParser<'static>>,
}

/// Creates a new sexpr_parser_t for the provided text
/// 
/// The returned object must be freed with sexpr_parser_free()
#[ffi_export]
pub fn sexpr_parser_new(text: char_p::Ref<'static>) -> repr_c::Box<sexpr_parser> {
    Box::new(sexpr_parser{parser: Shared::new(SExprParser::new(text.to_str()))}).into()
}

/// Frees a sexpr_parser_t
#[ffi_export]
pub fn sexpr_parser_free(parser: repr_c::Box<sexpr_parser>) {
    drop(parser);
}

/// Parses an atom from the text using the provided sexpr_parser_t and tokenizer_t
/// 
/// If a non-null value is returned, the returned atom must be freed with atom_free() or
/// provided to another function that takes ownership of the atom.
#[ffi_export]
pub fn sexpr_parser_parse(parser: &mut sexpr_parser, tokenizer: &tokenizer) -> Option<repr_c::Box<atom>> {
    parser.parser.borrow_mut().parse(&tokenizer.tok.borrow()).map(|atom| Box::new(atom{atom: atom}).into())
}

/// Returns an atom_t that represents a specific type
/// 
/// The returned atom must be freed with atom_free() or provided to another function that
/// takes ownership of the atom.
#[ffi_export] pub fn METTA_TYPE_UNDEFINED() -> repr_c::Box<atom> { Box::new(atom{atom: hyperon::metta::ATOM_TYPE_UNDEFINED}).into() }
#[ffi_export] pub fn METTA_TYPE_TYPE() -> repr_c::Box<atom> { Box::new(atom{atom: hyperon::metta::ATOM_TYPE_TYPE}).into() }
#[ffi_export] pub fn METTA_TYPE_ATOM() -> repr_c::Box<atom> { Box::new(atom{atom: hyperon::metta::ATOM_TYPE_ATOM}).into() }
#[ffi_export] pub fn METTA_TYPE_SYMBOL() -> repr_c::Box<atom> { Box::new(atom{atom: hyperon::metta::ATOM_TYPE_SYMBOL}).into() }
#[ffi_export] pub fn METTA_TYPE_VARIABLE() -> repr_c::Box<atom> { Box::new(atom{atom: hyperon::metta::ATOM_TYPE_VARIABLE}).into() }
#[ffi_export] pub fn METTA_TYPE_EXPRESSION() -> repr_c::Box<atom> { Box::new(atom{atom: hyperon::metta::ATOM_TYPE_EXPRESSION}).into() }
#[ffi_export] pub fn METTA_TYPE_GROUNDED() -> repr_c::Box<atom> { Box::new(atom{atom: hyperon::metta::ATOM_TYPE_GROUNDED}).into() }

/// Returns `true` if the supplied atom in the supplied space matches the supplied type,
/// otherwise returns `false`
#[ffi_export]
pub fn check_type(space: &grounding_space, atom: &atom, typ: &atom) -> bool {
    hyperon::metta::types::check_type(&space.space, &atom.atom, &typ.atom)
}

/// <TODO: stub documentation>
#[ffi_export]
pub fn validate_atom(space: &grounding_space, atom: &atom) -> bool {
    hyperon::metta::types::validate_atom(&space.space, &atom.atom)
}

/// Iterates all associated types for a provided atom, calling the `callback` function for each
#[ffi_export]
pub fn get_atom_types(space: &grounding_space, atom: &atom, callback: atoms_callback_t, context: *mut c_void) {
    let result_atoms = hyperon::metta::types::get_atom_types(&space.space, &atom.atom);
    for atom in result_atoms {
        callback((&atom as *const Atom).cast(), context);
    }
}

//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-
// MeTTa interpreter API.  metta_t Functions & Struct
//-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-=-+-

/// <TODO: stub documentation>
#[ffi_export]
#[derive_ReprC]
#[ReprC::opaque]
pub struct step_result {
    pub(crate) step: StepResult<'static, Vec<InterpretedAtom>, (Atom, Atom)>,
}

/// Creates a new step_result_t to interpret a provided expression within a provided space
/// 
/// <TODO: stub documentation>
/// 
/// The returned object needs to ultimately be passed to step_get_result() to free the object
#[ffi_export]
pub fn interpret_init(space: &'static mut grounding_space, expr: &atom) -> repr_c::Box<step_result> {
    let step = interpreter::interpret_init(&space.space, &expr.atom);
    Box::new(step_result{step: step}).into()
}

/// Advances the interpreter by one step
/// 
/// <TODO: stub documentation>
/// 
/// This function destroys the provided step object, but returns a new one in its place.
/// Therefore, it should be called like:
/// ```
/// step = interpret_step(step);
/// ```
#[ffi_export]
pub fn interpret_step(step: repr_c::Box<step_result>) -> repr_c::Box<step_result> {
    let next = interpreter::interpret_step(step.into().step);
    Box::new(step_result{step: next}).into()
}

/// Returns `true` if the interpreter has more work to do, or `false` if the interpreter
/// has reached a final state
/// 
/// <TODO: stub documentation>
#[ffi_export]
pub fn step_has_next(step: &step_result) -> bool {
    step.step.has_next()
}

/// <TODO: stub documentation>
/// 
/// Consumes the step_result_t, and calls the `callback` function for each resulting atom produced
/// by the interpreter
#[ffi_export]
pub fn step_get_result(step: repr_c::Box<step_result>, callback: atoms_callback_t, context: *mut c_void) {
    match step.into().step {
        StepResult::Return(mut res) => {
            let result_atoms: Vec<Atom> = res.drain(0..).map(|res| res.into_tuple().0).collect();
            for atom in result_atoms {
                callback((&atom as *const Atom).cast(), context);
            }
        },
        StepResult::Error(_) => (),
        StepResult::Execute(res) => panic!("Not expected step result: {:?}", res),
    }
}

/// Returns a string description of the step_result_t
/// 
/// The returned string needs to be freed with hyp_string_free()
#[ffi_export]
pub fn step_to_str(step: &step_result) -> char_p::Box {
    CString::new(format!("{:?}", step.step)).unwrap().into()
}

/// A top-level MeTTa runtime object
/// 
/// <TODO: stub documentation>
#[ffi_export]
#[derive_ReprC]
#[ReprC::opaque]
pub struct metta {
    pub(crate) metta: Shared<Metta>,
}

/// Creates a new metta_t
/// 
/// The returned object needs to be freed with metta_free()
#[ffi_export]
pub fn metta_new(space: &mut grounding_space, tokenizer: &mut tokenizer, cwd: char_p::Ref<'_>) -> repr_c::Box<metta> {
    let metta = Metta::from_space_cwd(space.space.clone(), tokenizer.tok.clone(), PathBuf::from(cwd.to_str()));
    Box::new(metta{metta: Shared::new(metta)}).into()
}

/// Clones a metta_t
/// 
/// The returned object needs to be freed with metta_free()
#[ffi_export]
pub fn metta_clone(metta: &metta) -> repr_c::Box<metta> {
    Box::new(metta{metta: metta.metta.clone()}).into()
}

/// Frees a metta_t
#[ffi_export]
pub fn metta_free(metta: repr_c::Box<metta>) {
    drop(metta);
}

/// Returns the grounding space associated with a metta_t
/// 
/// The returned object must be freed with grounding_space_free()
#[ffi_export]
pub fn metta_space(metta: &metta) -> repr_c::Box<grounding_space> {
    Box::new(grounding_space{space: metta.metta.borrow().space()}).into()
}

/// Returns the tokenizer associated with a metta_t
/// 
/// The returned object must be freed with tokenizer_free()
#[ffi_export]
pub fn metta_tokenizer(metta: &metta) -> repr_c::Box<tokenizer> {
    Box::new(tokenizer{tok: metta.metta.borrow().tokenizer()}).into()
}

/// <TODO: stub documentation>
#[ffi_export]
pub fn metta_run(metta: &mut metta, parser: &mut sexpr_parser,
        callback: atoms_callback_t, context: *mut c_void) {

    let metta = metta.metta.borrow();
    let mut parser = parser.parser.borrow_mut();
    let results = metta.run(&mut parser).expect("Returning errors from C API is not implemented yet");

    // TODO: return erorrs properly after step_get_result() is changed to return errors.
    for atom_set in results {
        for atom in atom_set {
            callback((&atom as *const Atom).cast(), context);
        }
    }
}

/// Evaluates the supplied atom within the supplied metta_t
/// <TODO: stub documentation>
/// 
/// This function takes ownership of the supplied atom, so it should not subsequently be freed
#[ffi_export]
pub fn metta_evaluate_atom(metta: &mut metta, atom: repr_c::Box<atom>,
    callback: atoms_callback_t, context: *mut c_void) {

    let metta = metta.metta.borrow();
    let result_atoms = metta.evaluate_atom(atom.into().atom)
        .expect("Returning errors from C API is not implemented yet");

    for atom in result_atoms {
        callback((&atom as *const Atom).cast(), context);
    }
}

/// Loads a module at the specified file system path into the metta_t
/// <TODO: stub documentation>
#[ffi_export]
pub fn metta_load_module(metta: &mut metta, name: char_p::Ref<'_>) {
    // TODO: return erorrs properly
    metta.metta.borrow().load_module(PathBuf::from(name.to_str()))
        .expect("Returning errors from C API is not implemented yet");
}
