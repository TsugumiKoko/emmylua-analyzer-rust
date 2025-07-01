use emmylua_parser::LuaNameExpr;

use crate::{DbIndex, LuaDeclId, LuaInferCache, LuaType};

// pub fn get_type_at_flow(
//     db: &DbIndex,
//     cache: &mut LuaInferCache,
//     name_expr: LuaNameExpr,
//     decl_id: LuaDeclId,
//     decl_type: LuaType,
// ) -> InferResult {
//     let file_id = cache.get_file_id();
//     let Some(flow_tree) = db.get_flow_index().get_flow_tree(&file_id) else {
//         return Ok(decl_type);
//     };

//     let Some(flow_id) = flow_tree.get_flow_id(name_expr.get_syntax_id()) else {
//         return Ok(decl_type);
//     };

//     let key = CacheKey::FlowNode(decl_id, flow_id);
//     if let Some(cache_entry) = cache.get(&key) {
//         if let CacheEntry::ExprCache(narrow_type) = cache_entry {
//             return Ok(narrow_type.clone());
//         }
//     }

//     let mut narrow_tyoe = decl_type.clone();
// }
