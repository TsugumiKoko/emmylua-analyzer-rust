use emmylua_parser::{LuaChunk, LuaExpr};

use crate::{
    semantic::infer::infer_name::narrow::ResultTypeOrContinue, DbIndex, FlowNode, FlowTree,
    InferFailReason, LuaDeclId, LuaInferCache,
};

pub fn get_type_at_condition_flow(
    db: &DbIndex,
    tree: &FlowTree,
    cache: &mut LuaInferCache,
    root: &LuaChunk,
    decl_id: LuaDeclId,
    flow_node: &FlowNode,
    condition: LuaExpr,
) -> Result<ResultTypeOrContinue, InferFailReason> {
    Ok(ResultTypeOrContinue::Continue)
}
