// Copyright © Aptos Foundation

use crate::{abort_unless_feature_flag_enabled, abort_unless_arithmetics_enabled_for_structure, ark_unary_op_internal, natives::{
    cryptography::algebra::{
        abort_invariant_violated, gas::GasParameters,
        AlgebraContext, Structure, MOVE_ABORT_CODE_NOT_IMPLEMENTED
    },
    helpers::{SafeNativeContext, SafeNativeError, SafeNativeResult},
}, safe_borrow_element, safely_pop_arg, store_element, structure_from_ty_arg};
use move_core_types::gas_algebra::NumArgs;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};
use ark_ff::Field;
use crate::natives::cryptography::algebra::feature_flag_from_structure;

pub fn sqr_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_arithmetics_enabled_for_structure!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_unary_op_internal!(
            context,
            args,
            ark_bls12_381::Fr,
            square,
            gas_params.ark_bls12_381_fr_square * NumArgs::one()
        ),
        Some(Structure::BLS12381Fq12) => ark_unary_op_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            square,
            gas_params.ark_bls12_381_fq12_square * NumArgs::one()
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
