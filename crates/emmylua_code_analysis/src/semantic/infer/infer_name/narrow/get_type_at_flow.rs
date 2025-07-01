use emmylua_parser::{LuaAssignStat, LuaChunk, LuaNameExpr};

use crate::{
    semantic::infer::{infer_name::narrow::get_decl_type, InferResult},
    CacheEntry, CacheKey, DbIndex, FlowAntecedent, FlowId, FlowNode, FlowNodeKind, FlowTree,
    InferFailReason, LuaDeclId, LuaInferCache, LuaType,
};

pub fn get_type_at_flow(
    db: &DbIndex,
    tree: &FlowTree,
    cache: &mut LuaInferCache,
    root: &LuaChunk,
    decl_id: LuaDeclId,
    flow_id: FlowId,
) -> InferResult {
    let key = CacheKey::FlowNode(decl_id, flow_id);
    if let Some(cache_entry) = cache.get(&key) {
        if let CacheEntry::ExprCache(narrow_type) = cache_entry {
            return Ok(narrow_type.clone());
        }
    }

    let mut result_type = LuaType::Unknown;
    let mut antecedent_flow_id = flow_id;
    loop {
        let flow_node = tree
            .get_flow_node(antecedent_flow_id)
            .ok_or(InferFailReason::None)?;
        match &flow_node.kind {
            FlowNodeKind::Start | FlowNodeKind::Unreachable => {
                result_type = get_decl_type(db, decl_id)?;
                break;
            }
            FlowNodeKind::LoopLabel => {
                antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
            }
            FlowNodeKind::BranchLabel | FlowNodeKind::NamedLabel(_) => todo!(),
            FlowNodeKind::DeclPosition(position) => {
                if *position <= decl_id.position {
                    result_type = get_decl_type(db, decl_id)?;
                    break;
                } else {
                    antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
                }
            }
            FlowNodeKind::Assignment(assign_ptr) => {
                let assign_stat = assign_ptr.to_node(root).ok_or(InferFailReason::None)?;
                let opt_type = get_type_at_assign_stat(
                    db,
                    tree,
                    cache,
                    root,
                    decl_id,
                    antecedent_flow_id,
                    assign_stat,
                )?;

                if let Some(assign_type) = opt_type {
                    result_type = assign_type;
                    break;
                } else {
                    antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
                }
            }
            FlowNodeKind::TrueCondition(lua_ast_ptr) => todo!(),
            FlowNodeKind::FalseCondition(lua_ast_ptr) => todo!(),
            FlowNodeKind::ForIStat(lua_ast_ptr) => todo!(),
            FlowNodeKind::TagCast(lua_ast_ptr) => todo!(),
            FlowNodeKind::AssertCall(lua_ast_ptr) => todo!(),
            FlowNodeKind::Break => todo!(),
            FlowNodeKind::Return => todo!(),
        }
    }

    let value = CacheEntry::ExprCache(result_type.clone());
    cache.add_cache(&key, value);
    Ok(result_type)
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

fn get_type_at_assign_stat(
    db: &DbIndex,
    tree: &FlowTree,
    cache: &mut LuaInferCache,
    root: &LuaChunk,
    decl_id: LuaDeclId,
    flow_id: FlowId,
    assign_stat: LuaAssignStat,
) -> Result<Option<LuaType>, InferFailReason> {
    todo!()
}
