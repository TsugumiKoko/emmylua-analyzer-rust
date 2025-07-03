use emmylua_parser::{BinaryOperator, LuaAstNode, LuaChunk, LuaDocTagCast, LuaExpr};

use crate::{
    semantic::infer::infer_name::narrow::{
        get_single_antecedent, get_type_at_flow, ResultTypeOrContinue,
    },
    DbIndex, FileId, FlowNode, FlowTree, InFiled, InferFailReason, LuaDeclId, LuaInferCache,
    LuaType, LuaTypeOwner, TypeOps,
};

pub fn get_type_at_cast_flow(
    db: &DbIndex,
    tree: &FlowTree,
    cache: &mut LuaInferCache,
    root: &LuaChunk,
    decl_id: LuaDeclId,
    flow_node: &FlowNode,
    tag_cast: LuaDocTagCast,
) -> Result<ResultTypeOrContinue, InferFailReason> {
    let key_expr = match tag_cast.get_key_expr() {
        Some(expr) => expr,
        None => return Ok(ResultTypeOrContinue::Continue),
    };

    // todo support index_expr
    let LuaExpr::NameExpr(name_expr) = key_expr else {
        return Ok(ResultTypeOrContinue::Continue);
    };
    let file_id = cache.get_file_id();
    let decl_tree = match db.get_decl_index().get_decl_tree(&file_id) {
        Some(tree) => tree,
        None => return Ok(ResultTypeOrContinue::Continue),
    };
    let name_text = match name_expr.get_name_text() {
        Some(text) => text,
        None => return Ok(ResultTypeOrContinue::Continue),
    };

    let ref_decl = match decl_tree.find_local_decl(&name_text, tag_cast.get_position()) {
        Some(decl_id) => decl_id,
        None => return Ok(ResultTypeOrContinue::Continue),
    };

    if ref_decl.get_id() != decl_id {
        return Ok(ResultTypeOrContinue::Continue);
    }

    let antecedent_flow_id = get_single_antecedent(tree, flow_node)?;
    let antecedent_type = get_type_at_flow(db, tree, cache, root, decl_id, antecedent_flow_id)?;
    let cast_result = cast_type(db, cache.get_file_id(), &tag_cast, antecedent_type)?;
    Ok(ResultTypeOrContinue::Result(cast_result))
}

enum CastAction {
    Add,
    Remove,
    Force,
}

fn cast_type(
    db: &DbIndex,
    file_id: FileId,
    tag_cast: &LuaDocTagCast,
    mut source_type: LuaType,
) -> Result<LuaType, InferFailReason> {
    for cast_op_type in tag_cast.get_op_types() {
        let action = match cast_op_type.get_op() {
            Some(op) => {
                if op.get_op() == BinaryOperator::OpAdd {
                    CastAction::Add
                } else {
                    CastAction::Remove
                }
            }
            None => CastAction::Force,
        };
        if cast_op_type.is_nullable() {
            match action {
                CastAction::Add => {
                    source_type = TypeOps::Union.apply(db, &source_type, &LuaType::Nil);
                }
                CastAction::Remove => {
                    source_type = TypeOps::Remove.apply(db, &source_type, &LuaType::Nil);
                }
                _ => {}
            }
        } else if let Some(doc_type) = cast_op_type.get_type() {
            let type_owner = LuaTypeOwner::SyntaxId(InFiled {
                file_id,
                value: doc_type.get_syntax_id(),
            });
            let typ = match db.get_type_index().get_type_cache(&type_owner) {
                Some(type_cache) => type_cache.as_type().clone(),
                None => {
                    continue;
                }
            };
            match action {
                CastAction::Add => {
                    source_type = TypeOps::Union.apply(db, &source_type, &typ);
                }
                CastAction::Remove => {
                    source_type = TypeOps::Remove.apply(db, &source_type, &typ);
                }
                CastAction::Force => {
                    source_type = typ;
                }
            }
        }
    }

    Ok(source_type)
}
