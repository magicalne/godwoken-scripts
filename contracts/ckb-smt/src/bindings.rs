/* automatically generated by rust-bindgen 0.59.1 */

pub const SMTErrorCode_ERROR_INSUFFICIENT_CAPACITY: SMTErrorCode = 80;
pub const SMTErrorCode_ERROR_NOT_FOUND: SMTErrorCode = 81;
pub const SMTErrorCode_ERROR_INVALID_STACK: SMTErrorCode = 82;
pub const SMTErrorCode_ERROR_INVALID_SIBLING: SMTErrorCode = 83;
pub const SMTErrorCode_ERROR_INVALID_PROOF: SMTErrorCode = 84;
pub type SMTErrorCode = cty::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct smt_pair_t {
    pub key: [u8; 32usize],
    pub value: [u8; 32usize],
    pub order: u32,
}
#[test]
fn bindgen_test_layout_smt_pair_t() {
    assert_eq!(
        ::core::mem::size_of::<smt_pair_t>(),
        68usize,
        concat!("Size of: ", stringify!(smt_pair_t))
    );
    assert_eq!(
        ::core::mem::align_of::<smt_pair_t>(),
        4usize,
        concat!("Alignment of ", stringify!(smt_pair_t))
    );
    assert_eq!(
        unsafe { &(*(::core::ptr::null::<smt_pair_t>())).key as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(smt_pair_t),
            "::",
            stringify!(key)
        )
    );
    assert_eq!(
        unsafe { &(*(::core::ptr::null::<smt_pair_t>())).value as *const _ as usize },
        32usize,
        concat!(
            "Offset of field: ",
            stringify!(smt_pair_t),
            "::",
            stringify!(value)
        )
    );
    assert_eq!(
        unsafe { &(*(::core::ptr::null::<smt_pair_t>())).order as *const _ as usize },
        64usize,
        concat!(
            "Offset of field: ",
            stringify!(smt_pair_t),
            "::",
            stringify!(order)
        )
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct smt_state_t {
    pub pairs: *mut smt_pair_t,
    pub len: u32,
    pub capacity: u32,
}
#[test]
fn bindgen_test_layout_smt_state_t() {
    assert_eq!(
        ::core::mem::size_of::<smt_state_t>(),
        16usize,
        concat!("Size of: ", stringify!(smt_state_t))
    );
    assert_eq!(
        ::core::mem::align_of::<smt_state_t>(),
        8usize,
        concat!("Alignment of ", stringify!(smt_state_t))
    );
    assert_eq!(
        unsafe { &(*(::core::ptr::null::<smt_state_t>())).pairs as *const _ as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(smt_state_t),
            "::",
            stringify!(pairs)
        )
    );
    assert_eq!(
        unsafe { &(*(::core::ptr::null::<smt_state_t>())).len as *const _ as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(smt_state_t),
            "::",
            stringify!(len)
        )
    );
    assert_eq!(
        unsafe { &(*(::core::ptr::null::<smt_state_t>())).capacity as *const _ as usize },
        12usize,
        concat!(
            "Offset of field: ",
            stringify!(smt_state_t),
            "::",
            stringify!(capacity)
        )
    );
}
extern "C" {
    pub fn smt_state_init(state: *mut smt_state_t, buffer: *mut smt_pair_t, capacity: u32);
}
extern "C" {
    pub fn smt_state_insert(
        state: *mut smt_state_t,
        key: *const u8,
        value: *const u8,
    ) -> cty::c_int;
}
extern "C" {
    pub fn smt_state_fetch(
        state: *mut smt_state_t,
        key: *const u8,
        value: *mut u8,
    ) -> cty::c_int;
}
extern "C" {
    pub fn smt_state_normalize(state: *mut smt_state_t);
}
extern "C" {
    pub fn smt_calculate_root(
        buffer: *mut u8,
        pairs: *const smt_state_t,
        proof: *const u8,
        proof_length: u32,
    ) -> cty::c_int;
}
extern "C" {
    pub fn smt_verify(
        hash: *const u8,
        state: *const smt_state_t,
        proof: *const u8,
        proof_length: u32,
    ) -> cty::c_int;
}
