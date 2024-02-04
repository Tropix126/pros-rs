#![allow(non_camel_case_types)]

use core::ffi::{c_int, c_void};

use super::{eh::EHAction, personality::find_eh_action};

pub(crate) const UNWIND_PRIVATE_DATA_SIZE: usize = 20; // https://dingelish.github.io/sgx_tunittest/src/sgx_unwind/libunwind.rs.html
pub(crate) const UNWIND_POINTER_REG: c_int = 12;
pub(crate) const UNWIND_DATA_REG: (i32, i32) = (0, 1); // https://github.com/llvm/llvm-project/blob/main/llvm/lib/Target/ARM/ARMISelLowering.cpp#L22022
pub(crate) const UNWIND_SP_REG: c_int = 13;
pub(crate) const UNWIND_IP_REG: c_int = 15;

#[repr(C)]
#[derive(Copy, Clone, PartialEq)]
pub(crate) enum _Unwind_Action {
    _UA_SEARCH_PHASE = 1,
    _UA_CLEANUP_PHASE = 2,
    _UA_HANDLER_FRAME = 4,
    _UA_FORCE_UNWIND = 8,
    _UA_END_OF_STACK = 16,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum _Unwind_Reason_Code {
    _URC_NO_REASON = 0,
    _URC_FOREIGN_EXCEPTION_CAUGHT = 1,
    _URC_FATAL_PHASE2_ERROR = 2,
    _URC_FATAL_PHASE1_ERROR = 3,
    _URC_NORMAL_STOP = 4,
    _URC_END_OF_STACK = 5,
    _URC_HANDLER_FOUND = 6,
    _URC_INSTALL_CONTEXT = 7,
    _URC_CONTINUE_UNWIND = 8,
    _URC_FAILURE = 9, // used only by ARM EHABI
}

#[repr(C)]
pub(crate) enum _Unwind_State {
    _US_VIRTUAL_UNWIND_FRAME = 0,
    _US_UNWIND_FRAME_STARTING = 1,
    _US_UNWIND_FRAME_RESUME = 2,
    _US_ACTION_MASK = 3,
    _US_FORCE_UNWIND = 8,
    _US_END_OF_STACK = 16,
}

pub(crate) type _Unwind_Exception_Class = u64;
pub(crate) type _Unwind_Word = *const u8;
pub(crate) type _Unwind_Ptr = *const u8;

pub(crate) type _Unwind_Exception_Cleanup_Fn =
    extern "C" fn(unwind_code: _Unwind_Reason_Code, exception: *mut _Unwind_Exception);

#[repr(C)]
pub(crate) struct _Unwind_Exception {
    pub exception_class: _Unwind_Exception_Class,
    pub exception_cleanup: _Unwind_Exception_Cleanup_Fn,
    pub private: [_Unwind_Word; UNWIND_PRIVATE_DATA_SIZE],
}

pub(crate) enum _Unwind_Context {}

#[repr(C)]
enum _Unwind_VRS_DataRepresentation {
	_UVRSD_UINT32 = 0,
	_UVRSD_VFPX = 1,
	_UVRSD_FPAX = 2,
	_UVRSD_UINT64 = 3,
	_UVRSD_FLOAT = 4,
	_UVRSD_DOUBLE = 5,
}

#[repr(C)]
enum _Unwind_VRS_RegClass {
	_UVRSC_CORE = 0,
	_UVRSC_VFP = 1,
	_UVRSC_FPA = 2,
	_UVRSC_WMMXD = 3,
	_UVRSC_WMMXC = 4,
}

#[repr(C)]
enum _Unwind_VRS_Result {
	_UVRSR_OK = 0,
	_UVRSR_NOT_IMPLEMENTED = 1,
	_UVRSR_FAILED = 2,
}

#[lang = "eh_personality"]
#[no_mangle]
unsafe extern "C" fn rust_eh_personality(
    state: _Unwind_State,
    exception_object: *mut _Unwind_Exception,
    context: *mut _Unwind_Context,
) -> _Unwind_Reason_Code {
    let state = state as c_int;
    let action = state & _Unwind_State::_US_ACTION_MASK as c_int;
    let search_phase = if action == _Unwind_State::_US_VIRTUAL_UNWIND_FRAME as c_int {
        // Backtraces on ARM will call the personality routine with
        // state == _US_VIRTUAL_UNWIND_FRAME | _US_FORCE_UNWIND. In those cases
        // we want to continue unwinding the stack, otherwise all our backtraces
        // would end at __rust_try
        if state & _Unwind_State::_US_FORCE_UNWIND as c_int != 0 {
            return continue_unwind(exception_object, context);
        }
        true
    } else if action == _Unwind_State::_US_UNWIND_FRAME_STARTING as c_int {
        false
    } else if action == _Unwind_State::_US_UNWIND_FRAME_RESUME as c_int {
        return continue_unwind(exception_object, context);
    } else {
        return _Unwind_Reason_Code::_URC_FAILURE;
    };

    // The DWARF unwinder assumes that _Unwind_Context holds things like the function
    // and LSDA pointers, however ARM EHABI places them into the exception object.
    // To preserve signatures of functions like _Unwind_GetLanguageSpecificData(), which
    // take only the context pointer, GCC personality routines stash a pointer to exception_object
    // in the context, using location reserved for ARM's "scratch register" (r12).
    _Unwind_SetGR(context, UNWIND_POINTER_REG, exception_object as _Unwind_Ptr);
    // ...A more principled approach would be to provide the full definition of ARM's
    // _Unwind_Context in our libunwind bindings and fetch the required data from there directly,
    // bypassing DWARF compatibility functions.

    let eh_action = match find_eh_action(context) {
        Ok(action) => action,
        Err(_) => return _Unwind_Reason_Code::_URC_FAILURE,
    };
    if search_phase {
        match eh_action {
            EHAction::None | EHAction::Cleanup(_) => {
                return continue_unwind(exception_object, context);
            }
            EHAction::Catch(_) | EHAction::Filter(_) => {
                // EHABI requires the personality routine to update the
                // SP value in the barrier cache of the exception object.
                (*exception_object).private[5] = _Unwind_GetGR(context, UNWIND_SP_REG);
                return _Unwind_Reason_Code::_URC_HANDLER_FOUND;
            }
            EHAction::Terminate => return _Unwind_Reason_Code::_URC_FAILURE,
        }
    } else {
        match eh_action {
            EHAction::None => return continue_unwind(exception_object, context),
            EHAction::Filter(_) if state & _Unwind_State::_US_FORCE_UNWIND as c_int != 0 => {
                return continue_unwind(exception_object, context)
            }
            EHAction::Cleanup(lpad) | EHAction::Catch(lpad) | EHAction::Filter(lpad) => {
                _Unwind_SetGR(context, UNWIND_DATA_REG.0, exception_object as _Unwind_Ptr);
                _Unwind_SetGR(context, UNWIND_DATA_REG.1, core::ptr::null());
                _Unwind_SetIP(context, lpad);
                return _Unwind_Reason_Code::_URC_INSTALL_CONTEXT;
            }
            EHAction::Terminate => return _Unwind_Reason_Code::_URC_FAILURE,
        }
    }

    // On ARM EHABI the personality routine is responsible for actually
    // unwinding a single stack frame before returning (ARM EHABI Sec. 6.1).
    unsafe fn continue_unwind(
        exception_object: *mut _Unwind_Exception,
        context: *mut _Unwind_Context,
    ) -> _Unwind_Reason_Code {
        if __gnu_unwind_frame(exception_object, context) == _Unwind_Reason_Code::_URC_NO_REASON {
            _Unwind_Reason_Code::_URC_CONTINUE_UNWIND
        } else {
            _Unwind_Reason_Code::_URC_FAILURE
        }
    }
}

extern "C" {
    pub(crate) fn _Unwind_GetLanguageSpecificData(ctx: *mut _Unwind_Context) -> *mut c_void;
    pub(crate) fn _Unwind_GetRegionStart(ctx: *mut _Unwind_Context) -> _Unwind_Ptr;
    pub(crate) fn _Unwind_GetTextRelBase(ctx: *mut _Unwind_Context) -> _Unwind_Ptr;
    pub(crate) fn _Unwind_GetDataRelBase(ctx: *mut _Unwind_Context) -> _Unwind_Ptr;
	fn _Unwind_VRS_Get(ctx: *mut _Unwind_Context,
		regclass: _Unwind_VRS_RegClass,
		regno: _Unwind_Word,
		repr: _Unwind_VRS_DataRepresentation,
		data: *mut c_void)
		-> _Unwind_VRS_Result;

fn _Unwind_VRS_Set(ctx: *mut _Unwind_Context,
		regclass: _Unwind_VRS_RegClass,
		regno: _Unwind_Word,
		repr: _Unwind_VRS_DataRepresentation,
		data: *mut c_void)
		-> _Unwind_VRS_Result;
    pub(crate) fn __gnu_unwind_frame(
        exception_object: *mut _Unwind_Exception,
        context: *mut _Unwind_Context,
    ) -> _Unwind_Reason_Code;
}

pub unsafe fn _Unwind_GetGR(ctx: *mut _Unwind_Context, reg_index: c_int) -> _Unwind_Word {
	let mut val: _Unwind_Word = core::ptr::null();
	_Unwind_VRS_Get(ctx, _Unwind_VRS_RegClass::_UVRSC_CORE, reg_index as _Unwind_Word, _Unwind_VRS_DataRepresentation::_UVRSD_UINT32,
					&mut val as *mut _ as *mut c_void);
	val
}

pub unsafe fn _Unwind_SetGR(ctx: *mut _Unwind_Context, reg_index: c_int, value: _Unwind_Word) {
	let mut value = value;
	_Unwind_VRS_Set(ctx, _Unwind_VRS_RegClass::_UVRSC_CORE, reg_index as _Unwind_Word, _Unwind_VRS_DataRepresentation::_UVRSD_UINT32,
					&mut value as *mut _ as *mut c_void);
}

pub unsafe fn _Unwind_GetIP(ctx: *mut _Unwind_Context) -> _Unwind_Word {
    let val = _Unwind_GetGR(ctx, UNWIND_IP_REG);
    val.map_addr(|v| v & !1)
}

pub unsafe fn _Unwind_GetIPInfo(ctx: *mut _Unwind_Context,
	ip_before_insn: *mut c_int)
	-> _Unwind_Word {
*ip_before_insn = 0;
_Unwind_GetIP(ctx)
}

pub unsafe fn _Unwind_SetIP(ctx: *mut _Unwind_Context,
	value: _Unwind_Word) {
// Propagate thumb bit to instruction pointer
let thumb_state = _Unwind_GetGR(ctx, UNWIND_IP_REG).addr() & 1;
let value = value.map_addr(|v| v | thumb_state);
_Unwind_SetGR(ctx, UNWIND_IP_REG, value);
}
