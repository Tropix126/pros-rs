use super::unwind;
use super::unwind::_Unwind_Context;
use super::eh;
use super::eh::{EHAction, EHContext};
use core::ffi::c_int;

pub(crate) unsafe fn find_eh_action(context: *mut _Unwind_Context) -> Result<EHAction, ()> {
    let lsda = unwind::_Unwind_GetLanguageSpecificData(context) as *const u8;
    let mut ip_before_instr: c_int = 0;
    let ip = unwind::_Unwind_GetIPInfo(context, &mut ip_before_instr);
    let eh_context = EHContext {
        // The return address points 1 byte past the call instruction,
        // which could be in the next IP range in LSDA range table.
        //
        // `ip = -1` has special meaning, so use wrapping sub to allow for that
        ip: if ip_before_instr != 0 { ip } else { ip.wrapping_sub(1) },
        func_start: unwind::_Unwind_GetRegionStart(context),
        get_text_start: &|| unwind::_Unwind_GetTextRelBase(context),
        get_data_start: &|| unwind::_Unwind_GetDataRelBase(context),
    };
    eh::find_eh_action(lsda, &eh_context)
}
