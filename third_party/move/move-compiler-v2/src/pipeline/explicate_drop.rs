use super::{
    livevar_analysis_processor::{LiveVarAnnotation, LiveVarInfoAtCodeOffset},
    reference_safety_processor::{LifetimeAnnotation, LifetimeInfoAtCodeOffset},
};
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::FunctionTargetProcessor,
    stackless_bytecode::{AttrId, Bytecode, Operation},
};
use std::collections::BTreeSet;

pub struct ExplicateDrop {}

impl FunctionTargetProcessor for ExplicateDrop {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(fun_env, &data);
        let live_var_annotation = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("livevar annotation");
        let lifetime_annotation = target
            .get_annotations()
            .get::<LifetimeAnnotation>()
            .expect("lifetime annotation");
        let mut transformer = ExplicateDropTransformer {
            fun_env,
            fun_data: data,
            live_var_annot: live_var_annotation,
            lifetime_annot: lifetime_annotation,
        };
        transformer.transform();
        transformer.data
    }

    fn name(&self) -> String {
        "ExplicateDrop".to_owned()
    }
}

struct ExplicateDropTransformer<'a> {
    fun_env: &'a FunctionEnv<'a>,
    fun_data: FunctionData,
    live_var_annot: &'a LiveVarAnnotation,
    lifetime_annot: &'a LifetimeAnnotation,
}

impl<'a> ExplicateDropTransformer<'a> {
    // Add explicit drop instructions
    pub fn transform(&mut self) {
        let bytecodes = std::mem::take(&mut self.fun_data.code);
        for (code_offset, bytecode) in bytecodes.into_iter().enumerate() {
            let attr_id = bytecode.get_attr_id();
            self.emit_bytecode(bytecode);
            self.explicate_drops_at(code_offset, attr_id);
        }
    }

    // add drops at given code offset
    fn explicate_drops_at(&mut self, code_offest: CodeOffset, attr_id: AttrId) {
        let released_temps = self.released_temps_at(code_offset);
        for t in released_temps {
            let drop_t = Bytecode::Call(
                attr_id,
                Vec::new(),
                Operation::Destroy,
                vec![t],
                None
            );
            self.emit_bytecode(drop_t)
        }
    }

    // Returns a set of locals that can be dropped at given code offset
    fn released_temps_at(&self, code_offset: CodeOffset) -> BTreeSet<TempIndex> {
        let live_var_info = self
            .live_var_annot
            .get_live_var_info_at(code_offset)
            .expect("live var info");
        let lifetime_info = self
            .lifetime_annot
            .get_info_at(code_offset);
        released_temps(live_var_info, life_time_info)
    }

    fn emit_bytecode(&mut self, bytecode: Bytecode) {
        self.fun_data.code.push(bytecode)
    }
}

// Returns a set of locals that can be dropped
// these are the ones no longer alive or borrowed
fn released_temps(
    live_var_info: &LiveVarInfoAtCodeOffset,
    life_time_info: &LifetimeInfoAtCodeOffset,
) -> BTreeSet<TempIndex> {
    // use set to avoid duplicate dropping
    let mut released_temps = BTreeSet::new();
    for t in live_var_info.released_temps() {
        if !life_time_info.after.temp_is_borrowed(t) {
            released_temps.insert(t)
        }
    }
    for t in life_time_info.released_temps() {
        if !live_var_info.after.contains_key(key) {
            released_temps.insert(t)
        }
    }
    released_temps
}
