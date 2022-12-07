/// Declares the aggregate to be exported.
#[macro_export]
macro_rules! export_aggregate(($t:ident) => {
    const _: () = {
        #[doc(hidden)]
        #[export_name = "aggregate#init"]
        #[allow(non_snake_case)]
        unsafe extern "C" fn __export_aggregate_init(arg0: i32, arg1: i32) -> i32 {
            $crate::wit_aggregate::aggregate::call_init::<$t>(arg0, arg1)
        }
        #[doc(hidden)]
        #[export_name = "cabi_post_aggregate#init"]
        #[allow(non_snake_case)]
        unsafe extern "C" fn __post_return_aggregate_init(arg0: i32) {
            $crate::wit_aggregate::aggregate::post_return_init::<$t>(arg0)
        }
        #[doc(hidden)]
        #[export_name = "aggregate#apply"]
        #[allow(non_snake_case)]
        unsafe extern "C" fn __export_aggregate_apply(
            arg0: i32,
            arg1: i32,
            arg2: i32,
            arg3: i32,
        ) -> i32 {
            $crate::wit_aggregate::aggregate::call_apply::<$t>(
                arg0,
                arg1,
                arg2,
                arg3,
            )
        }
        #[doc(hidden)]
        #[export_name = "cabi_post_aggregate#apply"]
        #[allow(non_snake_case)]
        unsafe extern "C" fn __post_return_aggregate_apply(arg0: i32) {
            $crate::wit_aggregate::aggregate::post_return_apply::<$t>(arg0)
        }
        #[doc(hidden)]
        #[export_name = "aggregate#handle"]
        #[allow(non_snake_case)]
        unsafe extern "C" fn __export_aggregate_handle(
            arg0: i32,
            arg1: i32,
            arg2: i32,
            arg3: i32,
            arg4: i32,
            arg5: i32,
            arg6: i64,
            arg7: i64,
            arg8: i32,
            arg9: i32,
            arg10: i64,
            arg11: i32,
            arg12: i32,
            arg13: i32,
            arg14: i32,
        ) -> i32 {
            $crate::wit_aggregate::aggregate::call_handle::<$t>(
                arg0,
                arg1,
                arg2,
                arg3,
                arg4,
                arg5,
                arg6,
                arg7,
                arg8,
                arg9,
                arg10,
                arg11,
                arg12,
                arg13,
                arg14,
            )
        }
        #[doc(hidden)]
        #[export_name = "cabi_post_aggregate#handle"]
        #[allow(non_snake_case)]
        unsafe extern "C" fn __post_return_aggregate_handle(arg0: i32) {
                $crate::wit_aggregate::aggregate::post_return_handle::<$t>(arg0)
        }
    };
    #[used]
    #[doc(hidden)]
    #[cfg(target_arch = "wasm32")]
    static __FORCE_SECTION_REF: fn() = __force_section_ref;
    #[doc(hidden)]
    #[cfg(target_arch = "wasm32")]
    fn __force_section_ref() {
        $crate::wit_aggregate::__link_section()
    }
});

macro_rules! event_whitelist {
    ($aggregate:path, [ $( $event:path ),* $(,)? ]) => {[
        $( thalo_pg_eventstore::EventWhitelist {
            category: <$aggregate as thalo::Aggregate>::aggregate_type(),
            event_type: <$event as thalo::Event>::event_type(),
        }, )*
    ]};
}
