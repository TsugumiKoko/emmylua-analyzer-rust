mod get_type_at_flow;
mod get_type_at_cast_flow;
mod get_type_at_condition_flow;

use emmylua_parser::{LuaAstNode, LuaChunk, LuaNameExpr};

use crate::{
    infer_param, semantic::infer::{
        infer_name::{infer_global_type, narrow::get_type_at_flow::get_type_at_flow},
        InferResult,
    }, DbIndex, FlowAntecedent, FlowId, FlowNode, FlowTree, InferFailReason, LuaDeclId, LuaInferCache, LuaType
};

pub fn infer_name_expr_narrow_type(
    db: &DbIndex,
    cache: &mut LuaInferCache,
    name_expr: LuaNameExpr,
    decl_id: LuaDeclId,
) -> InferResult {
    let file_id = cache.get_file_id();
    let Some(flow_tree) = db.get_flow_index().get_flow_tree(&file_id) else {
        return get_decl_type(db, decl_id);
    };

    let Some(flow_id) = flow_tree.get_flow_id(name_expr.get_syntax_id()) else {
        return get_decl_type(db, decl_id);
    };

    let root = LuaChunk::cast(name_expr.get_root()).ok_or(InferFailReason::None)?;
    get_type_at_flow(db, flow_tree, cache, &root, decl_id, flow_id)
}

fn get_decl_type(db: &DbIndex, decl_id: LuaDeclId) -> InferResult {
    let decl = db
        .get_decl_index()
        .get_decl(&decl_id)
        .ok_or(InferFailReason::None)?;

    if decl.is_global() {
        let name = decl.get_name();
        return infer_global_type(db, name);
    }

    if let Some(type_cache) = db.get_type_index().get_type_cache(&decl.get_id().into()) {
        return Ok(type_cache.as_type().clone());
    }

    if decl.is_param() {
        return infer_param(db, decl);
    }

    Err(InferFailReason::UnResolveDeclType(decl.get_id()))
}

fn get_single_antecedent(tree: &FlowTree, flow: &FlowNode) -> Result<FlowId, InferFailReason> {
    match &flow.antecedent {
        Some(antecedent) => match antecedent {
            FlowAntecedent::Single(id) => Ok(*id),
            FlowAntecedent::Multiple(multi_id) => {
                let multi_flow = tree
                    .get_multi_antecedents(*multi_id)
                    .ok_or(InferFailReason::None)?;
                if multi_flow.len() > 0 {
                    // If there are multiple antecedents, we need to handle them separately
                    // For now, we just return the first one
                    Ok(multi_flow[0])
                } else {
                    Err(InferFailReason::None)
                }
            }
        },
        None => Err(InferFailReason::None),
    }
}

#[derive(Debug)]
pub enum ResultTypeOrContinue {
    Result(LuaType),
    Continue,
}