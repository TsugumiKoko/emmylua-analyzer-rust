use emmylua_parser::{LuaAssignStat, LuaAstNode, LuaCallExpr, LuaChunk, LuaExpr};

use crate::{
    infer_expr,
    semantic::infer::{
        infer_name::narrow::{
            get_decl_type, get_single_antecedent, get_type_at_cast_flow::get_type_at_cast_flow,
            get_type_at_condition_flow::get_type_at_condition_flow, ResultTypeOrContinue,
        },
        InferResult,
    },
    CacheEntry, CacheKey, DbIndex, FlowId, FlowNode, FlowNodeKind, FlowTree, InferFailReason,
    LuaDeclId, LuaInferCache, LuaType, TypeOps,
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

    #[allow(unused_mut)]
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
            FlowNodeKind::LoopLabel | FlowNodeKind::Break | FlowNodeKind::Return => {
                antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
            }
            FlowNodeKind::BranchLabel | FlowNodeKind::NamedLabel(_) => {
                // todo support many branch
                antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
            }
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
                let result_or_continue = get_type_at_assign_stat(
                    db,
                    tree,
                    cache,
                    root,
                    decl_id,
                    flow_node,
                    assign_stat,
                )?;

                if let ResultTypeOrContinue::Result(assign_type) = result_or_continue {
                    result_type = assign_type;
                    break;
                } else {
                    antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
                }
            }
            FlowNodeKind::TrueCondition(condition_ptr) => {
                let condition = condition_ptr.to_node(root).ok_or(InferFailReason::None)?;
                let result_or_continue = get_type_at_condition_flow(
                    db, tree, cache, root, decl_id, flow_node, condition,
                )?;

                if let ResultTypeOrContinue::Result(condition_type) = result_or_continue {
                    result_type = condition_type;
                    break;
                } else {
                    antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
                }
            }
            FlowNodeKind::FalseCondition(_) => {
                // todo support
                antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
            },
            FlowNodeKind::ForIStat(_) => {
                // todo check for `for i = 1, 10 do end`
                antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
            }
            FlowNodeKind::TagCast(cast_ast_ptr) => {
                let tag_cast = cast_ast_ptr.to_node(root).ok_or(InferFailReason::None)?;
                let cast_or_continue =
                    get_type_at_cast_flow(db, tree, cache, root, decl_id, flow_node, tag_cast)?;

                if let ResultTypeOrContinue::Result(cast_type) = cast_or_continue {
                    result_type = cast_type;
                    break;
                } else {
                    antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
                }
            }
            FlowNodeKind::AssertCall(lua_ast_ptr) => {
                let assert_call = lua_ast_ptr.to_node(root).ok_or(InferFailReason::None)?;
                let result_or_continue = get_type_at_assert_call(
                    db,
                    tree,
                    cache,
                    root,
                    decl_id,
                    flow_node,
                    assert_call,
                )?;

                if let ResultTypeOrContinue::Result(assert_type) = result_or_continue {
                    result_type = assert_type;
                    break;
                } else {
                    antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
                }
            }
        }
    }

    let value = CacheEntry::ExprCache(result_type.clone());
    cache.add_cache(&key, value);
    Ok(result_type)
}

fn get_type_at_assign_stat(
    db: &DbIndex,
    tree: &FlowTree,
    cache: &mut LuaInferCache,
    root: &LuaChunk,
    decl_id: LuaDeclId,
    flow_node: &FlowNode,
    assign_stat: LuaAssignStat,
) -> Result<ResultTypeOrContinue, InferFailReason> {
    let (vars, exprs) = assign_stat.get_var_and_expr_list();
    let file_id = cache.get_file_id();
    for i in 0..vars.len() {
        let var = vars[i].clone();
        let range = var.get_range();
        let maybe_ref_id = LuaDeclId::new(file_id, range.start());
        if maybe_ref_id == decl_id {
            let typ = get_decl_type(db, decl_id)?;
            return Ok(ResultTypeOrContinue::Result(typ));
        }

        let ref_decl_id = db
            .get_reference_index()
            .get_var_reference_decl(&file_id, range);
        if let Some(ref_decl_id) = ref_decl_id {
            if ref_decl_id == decl_id {
                let expr_type = match exprs.get(i) {
                    Some(expr) => infer_expr(db, cache, expr.clone())?,
                    None => {
                        let expr_len = exprs.len();
                        if expr_len == 0 {
                            return Ok(ResultTypeOrContinue::Continue);
                        }

                        let last_expr = exprs[expr_len - 1].clone();
                        let last_expr_type = infer_expr(db, cache, last_expr)?;
                        if let LuaType::Variadic(variadic) = last_expr_type {
                            let idx = i - expr_len + 1;
                            match variadic.get_type(idx) {
                                Some(typ) => typ.clone(),
                                None => return Ok(ResultTypeOrContinue::Continue),
                            }
                        } else {
                            return Ok(ResultTypeOrContinue::Continue);
                        }
                    }
                };

                let antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
                let antecedent_type =
                    get_type_at_flow(db, tree, cache, root, ref_decl_id, antecedent_flow_id)?;

                return Ok(ResultTypeOrContinue::Result(TypeOps::Narrow.apply(
                    db,
                    &antecedent_type,
                    &expr_type,
                )));
            }
        }
    }

    Ok(ResultTypeOrContinue::Continue)
}

fn get_type_at_assert_call(
    db: &DbIndex,
    tree: &FlowTree,
    cache: &mut LuaInferCache,
    root: &LuaChunk,
    decl_id: LuaDeclId,
    flow_node: &FlowNode,
    assert_call: LuaCallExpr,
) -> Result<ResultTypeOrContinue, InferFailReason> {
    let file_id = cache.get_file_id();
    let call_arg_list = match assert_call.get_args_list() {
        Some(args) => args,
        None => return Ok(ResultTypeOrContinue::Continue),
    };

    for arg in call_arg_list.get_args() {
        if let LuaExpr::NameExpr(name_expr) = arg {
            let ref_decl_id = db
                .get_reference_index()
                .get_var_reference_decl(&file_id, name_expr.get_range());
            if let Some(ref_decl_id) = ref_decl_id {
                if ref_decl_id == decl_id {
                    let antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
                    let antecedent_type =
                        get_type_at_flow(db, tree, cache, root, ref_decl_id, antecedent_flow_id)?;
                    let result_type = TypeOps::RemoveNilOrFalse.apply_source(db, &antecedent_type);

                    return Ok(ResultTypeOrContinue::Result(result_type));
                }
            }
        }
        // todo for index_expr
    }

    Ok(ResultTypeOrContinue::Continue)
}
