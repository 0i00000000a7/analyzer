//! wasm-bindgen exports for the BMS analyzer.
//! Mirrors the 24 exports from src/cpp/bindings.cpp.

use bms_core::bms::{bms_to_bocf, is_eq_ebo, is_gte_ebo};
use bms_core::expand::{expand_bms, matrix_lex_order};
use bms_core::one_y::build_1y_mountain_with_rows;
use bms_core::parser::{bocf_to_bms, eval_ast, parse_bocf};
use bms_core::term::*;
use bms_core::triangular::{bms_to_triangular, triangular_to_bms};
use bms_core::wy::{
    build_wy_mountain_with_rows, expand_1y, expand_wy_seq,
};
use bms_core::y_dbms::{dbms_to_bms, dbms_to_one_y, dbms_to_string, one_y_to_dbms};
use bms_core::zero_y::{build_mountain, bms_to_0y_sequence, zero_y_expand, zero_y_to_bms};
use bms_core::Matrix;
use wasm_bindgen::prelude::*;

// ════════════════════════════════════════════════════════════════
// JS value conversion helpers
// ════════════════════════════════════════════════════════════════

fn js_to_matrix(js: &JsValue) -> Matrix {
    let arr = js_sys::Array::from(js);
    let len = arr.length();
    let mut m = Vec::with_capacity(len as usize);
    for i in 0..len {
        let col = js_sys::Array::from(&arr.get(i));
        let col_len = col.length();
        let mut row = Vec::with_capacity(col_len as usize);
        for j in 0..col_len {
            row.push(col.get(j).as_f64().unwrap_or(0.0) as i32);
        }
        while row.len() < 3 {
            row.push(0);
        }
        m.push(row);
    }
    m
}

/// Like js_to_matrix but preserves column lengths (DBMS columns are ragged).
fn js_to_matrix_no_pad(js: &JsValue) -> Matrix {
    let arr = js_sys::Array::from(js);
    let len = arr.length();
    let mut m = Vec::with_capacity(len as usize);
    for i in 0..len {
        let col = js_sys::Array::from(&arr.get(i));
        let col_len = col.length();
        let mut row = Vec::with_capacity(col_len as usize);
        for j in 0..col_len {
            row.push(col.get(j).as_f64().unwrap_or(0.0) as i32);
        }
        m.push(row);
    }
    m
}

fn matrix_to_js(m: &Matrix) -> JsValue {
    let arr = js_sys::Array::new();
    for col in m {
        let col_arr = js_sys::Array::new();
        for v in col {
            col_arr.push(&JsValue::from(*v));
        }
        arr.push(&col_arr);
    }
    arr.into()
}

fn term_to_js(t: &Term) -> JsValue {
    match t {
        None => js_sys::Array::new().into(),
        Some(n) => {
            let arr = js_sys::Array::new();
            arr.push(&term_to_js(&n.a));
            arr.push(&term_to_js(&n.b));
            arr.push(&term_to_js(&n.c));
            arr.into()
        }
    }
}

fn js_to_term(js: &JsValue) -> Term {
    let arr = js_sys::Array::from(js);
    if arr.length() == 0 {
        return zero();
    }
    let a = js_to_term(&arr.get(0));
    let b = js_to_term(&arr.get(1));
    let c = js_to_term(&arr.get(2));
    t(a, b, c)
}

fn js_seq_to_vec(js: &JsValue) -> Vec<i32> {
    let arr = js_sys::Array::from(js);
    let mut v = Vec::with_capacity(arr.length() as usize);
    for i in 0..arr.length() {
        v.push(arr.get(i).as_f64().unwrap_or(0.0) as i32);
    }
    v
}

fn vec_to_js_seq(v: &[i32]) -> JsValue {
    let arr = js_sys::Array::new();
    for x in v {
        arr.push(&JsValue::from(*x));
    }
    arr.into()
}

/// 0-Y mountain: nodes as {value, parent} objects (matches old embind output).
fn mountain_to_js(mountain: &bms_core::Mountain) -> JsValue {
    let arr = js_sys::Array::new();
    for layer in mountain {
        let layer_arr = js_sys::Array::new();
        for (value, parent) in layer {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &JsValue::from("value"), &JsValue::from(*value)).ok();
            js_sys::Reflect::set(&obj, &JsValue::from("parent"), &JsValue::from(*parent)).ok();
            layer_arr.push(&obj);
        }
        arr.push(&layer_arr);
    }
    arr.into()
}

/// WY/1-Y mountain: nodes as {value, parent, parentCol} objects. parentCol is
/// the absolute column of the parent (-1 when none), matching old embind output.
fn wy_mountain_to_js(mountain: &bms_core::Mountain) -> JsValue {
    let arr = js_sys::Array::new();
    for layer in mountain {
        let layer_arr = js_sys::Array::new();
        for (col, (value, parent)) in layer.iter().enumerate() {
            let parent_col = if *parent > 0 { col as i32 - *parent } else { -1 };
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(&obj, &JsValue::from("value"), &JsValue::from(*value)).ok();
            js_sys::Reflect::set(&obj, &JsValue::from("parent"), &JsValue::from(*parent)).ok();
            js_sys::Reflect::set(&obj, &JsValue::from("parentCol"), &JsValue::from(parent_col)).ok();
            layer_arr.push(&obj);
        }
        arr.push(&layer_arr);
    }
    arr.into()
}

fn wy_rows_to_js(rows: &[Vec<i32>]) -> JsValue {
    let arr = js_sys::Array::new();
    for row in rows {
        let row_arr = js_sys::Array::new();
        for v in row {
            row_arr.push(&JsValue::from(*v));
        }
        arr.push(&row_arr);
    }
    arr.into()
}

// ════════════════════════════════════════════════════════════════
// 24 exported functions
// ════════════════════════════════════════════════════════════════

#[wasm_bindgen(js_name = "bmsAnalyze")]
pub fn bms_analyze(matrix: JsValue) -> JsValue {
    let m = js_to_matrix(&matrix);
    let obj = js_sys::Object::new();
    let gte_ebo = is_gte_ebo(&m);
    js_sys::Reflect::set(&obj, &JsValue::from("gteEBO"), &JsValue::from(gte_ebo)).ok();
    if gte_ebo {
        let lb = if is_eq_ebo(&m) { "\\psi(I)" } else { ">\\psi(I)" };
        js_sys::Reflect::set(&obj, &JsValue::from("ordinal"), &JsValue::from(lb)).ok();
    } else {
        let ordinal = bms_to_bocf(&m);
        let s = term_to_string(false, &ordinal);
        let veblen = term_to_veblen(&ordinal);
        let veblen_plain = term_to_veblen_plain(&ordinal);
        let veblen_matrix = term_to_veblen_matrix(&ordinal);
        let veblen_matrix_plain = term_to_veblen_matrix_plain(&ordinal);
        let is_standard = true;

        js_sys::Reflect::set(&obj, &JsValue::from("ordinal"), &JsValue::from(&s)).ok();
        js_sys::Reflect::set(&obj, &JsValue::from("ordinalJS"), &term_to_js(&ordinal)).ok();
        js_sys::Reflect::set(&obj, &JsValue::from("veblen"), &JsValue::from(&veblen)).ok();
        js_sys::Reflect::set(&obj, &JsValue::from("veblenPlain"), &JsValue::from(&veblen_plain)).ok();
        js_sys::Reflect::set(&obj, &JsValue::from("veblenMatrix"), &JsValue::from(&veblen_matrix)).ok();
        js_sys::Reflect::set(&obj, &JsValue::from("veblenMatrixPlain"), &JsValue::from(&veblen_matrix_plain)).ok();
        js_sys::Reflect::set(&obj, &JsValue::from("nsForm"), &JsValue::from(&s)).ok();
        js_sys::Reflect::set(&obj, &JsValue::from("isStandard"), &JsValue::from(is_standard)).ok();
    }
    obj.into()
}

#[wasm_bindgen(js_name = "matrixLexOrder")]
pub fn matrix_lex_order_js(a: JsValue, b: JsValue) -> i32 {
    matrix_lex_order(&js_to_matrix(&a), &js_to_matrix(&b))
}

#[wasm_bindgen(js_name = "decomposePower")]
pub fn decompose_power_js(term: JsValue) -> JsValue {
    let (first, second) = decompose_power(&js_to_term(&term));
    let arr = js_sys::Array::new();
    arr.push(&term_to_js(&first));
    arr.push(&term_to_js(&second));
    arr.into()
}

#[wasm_bindgen(js_name = "computeT")]
pub fn compute_t_js(term: JsValue) -> JsValue {
    term_to_js(&compute_t(&js_to_term(&term)))
}

#[wasm_bindgen(js_name = "zeroYToBMS")]
pub fn zero_y_to_bms_js(seq: JsValue) -> JsValue {
    matrix_to_js(&zero_y_to_bms(&js_seq_to_vec(&seq)))
}

#[wasm_bindgen(js_name = "zeroYExpand")]
pub fn zero_y_expand_js(seq: JsValue, n: i32) -> JsValue {
    vec_to_js_seq(&zero_y_expand(&js_seq_to_vec(&seq), n))
}

#[wasm_bindgen(js_name = "parseAndEvalBOCF")]
pub fn parse_and_eval_bocf_js(input: &str) -> JsValue {
    let obj = js_sys::Object::new();
    match parse_bocf(input) {
        Err(e) => {
            js_sys::Reflect::set(&obj, &JsValue::from("ast"), &JsValue::from("")).ok();
            js_sys::Reflect::set(&obj, &JsValue::from("ordinal"), &JsValue::from("")).ok();
            js_sys::Reflect::set(&obj, &JsValue::from("ordinalJS"), &term_to_js(&zero())).ok();
            js_sys::Reflect::set(&obj, &JsValue::from("error"), &JsValue::from(&e)).ok();
        }
        Ok(ast) => match eval_ast(&ast) {
            Err(e) => {
                js_sys::Reflect::set(&obj, &JsValue::from("ast"), &JsValue::from("")).ok();
                js_sys::Reflect::set(&obj, &JsValue::from("ordinal"), &JsValue::from("")).ok();
                js_sys::Reflect::set(&obj, &JsValue::from("ordinalJS"), &term_to_js(&zero())).ok();
                js_sys::Reflect::set(&obj, &JsValue::from("error"), &JsValue::from(&e)).ok();
            }
            Ok(val) => {
                js_sys::Reflect::set(&obj, &JsValue::from("ast"), &JsValue::from("")).ok();
                js_sys::Reflect::set(&obj, &JsValue::from("ordinal"), &JsValue::from(term_to_string(false, &val))).ok();
                js_sys::Reflect::set(&obj, &JsValue::from("ordinalJS"), &term_to_js(&val)).ok();
                js_sys::Reflect::set(&obj, &JsValue::from("error"), &JsValue::from("")).ok();
            }
        },
    }
    obj.into()
}

#[wasm_bindgen(js_name = "expandBMS")]
pub fn expand_bms_js(matrix: JsValue, fs: i32) -> JsValue {
    matrix_to_js(&expand_bms(&js_to_matrix(&matrix), fs))
}

#[wasm_bindgen(js_name = "bmsTo0YSequence")]
pub fn bms_to_0y_sequence_js(matrix: JsValue) -> String {
    bms_to_0y_sequence(&js_to_matrix(&matrix))
}

#[wasm_bindgen(js_name = "subscriptDepth")]
pub fn subscript_depth_js(term: JsValue) -> i32 {
    subscript_depth(&js_to_term(&term))
}

#[wasm_bindgen(js_name = "termToVeblen")]
pub fn term_to_veblen_js(term: JsValue) -> JsValue {
    let t = js_to_term(&term);
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsValue::from("veblen"), &JsValue::from(term_to_veblen(&t))).ok();
    js_sys::Reflect::set(&obj, &JsValue::from("veblenPlain"), &JsValue::from(term_to_veblen_plain(&t))).ok();
    js_sys::Reflect::set(&obj, &JsValue::from("veblenMatrix"), &JsValue::from(term_to_veblen_matrix(&t))).ok();
    js_sys::Reflect::set(&obj, &JsValue::from("veblenMatrixPlain"), &JsValue::from(term_to_veblen_matrix_plain(&t))).ok();
    obj.into()
}

#[wasm_bindgen(js_name = "bocfToBMS")]
pub fn bocf_to_bms_js(input: &str, progress: &js_sys::Function) -> JsValue {
    let obj = js_sys::Object::new();
    match bocf_to_bms(input, &mut |s: &str| {
        let _ = progress.call1(&JsValue::null(), &JsValue::from(s));
    }) {
        Ok(result) => {
            js_sys::Reflect::set(&obj, &JsValue::from("result"), &JsValue::from(&result)).ok();
            js_sys::Reflect::set(&obj, &JsValue::from("error"), &JsValue::from("")).ok();
        }
        Err(e) => {
            js_sys::Reflect::set(&obj, &JsValue::from("result"), &JsValue::from("")).ok();
            js_sys::Reflect::set(&obj, &JsValue::from("error"), &JsValue::from(&e)).ok();
        }
    }
    obj.into()
}

#[wasm_bindgen(js_name = "fundamentalSequence")]
pub fn fundamental_sequence_js(term: JsValue, n: i32) -> JsValue {
    let result = fundamental_sequence(&js_to_term(&term), n);
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsValue::from("term"), &JsValue::from(term_to_string(false, &result))).ok();
    js_sys::Reflect::set(&obj, &JsValue::from("termJS"), &term_to_js(&result)).ok();
    obj.into()
}

#[wasm_bindgen(js_name = "cofinality")]
pub fn cofinality_js(term: JsValue) -> JsValue {
    let result = cofinality(&js_to_term(&term));
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsValue::from("term"), &JsValue::from(term_to_string(false, &result))).ok();
    js_sys::Reflect::set(&obj, &JsValue::from("termJS"), &term_to_js(&result)).ok();
    obj.into()
}

#[wasm_bindgen(js_name = "triangularToBMS")]
pub fn triangular_to_bms_js(matrix: JsValue) -> JsValue {
    matrix_to_js(&triangular_to_bms(&js_to_matrix(&matrix)))
}

#[wasm_bindgen(js_name = "bmsToTriangular")]
pub fn bms_to_triangular_js(matrix: JsValue) -> JsValue {
    matrix_to_js(&bms_to_triangular(&js_to_matrix(&matrix)))
}

#[wasm_bindgen(js_name = "buildMountain")]
pub fn build_mountain_js(seq: JsValue) -> JsValue {
    mountain_to_js(&build_mountain(&js_seq_to_vec(&seq)))
}

fn empty_mountain_result() -> JsValue {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsValue::from("layers"), &js_sys::Array::new()).ok();
    js_sys::Reflect::set(&obj, &JsValue::from("rows"), &js_sys::Array::new()).ok();
    obj.into()
}

#[wasm_bindgen(js_name = "buildWYMountain")]
pub fn build_wy_mountain_js(seq: JsValue, n: i32, consistent: bool) -> JsValue {
    let v = js_seq_to_vec(&seq);
    if v.is_empty() || v[0] == 0 || v.iter().any(|&x| x < 0) {
        return empty_mountain_result();
    }
    let (mountain, rows) = build_wy_mountain_with_rows(&v, n, consistent);
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsValue::from("layers"), &wy_mountain_to_js(&mountain)).ok();
    js_sys::Reflect::set(&obj, &JsValue::from("rows"), &wy_rows_to_js(&rows)).ok();
    obj.into()
}

#[wasm_bindgen(js_name = "build1YMountain")]
pub fn build_1y_mountain_js(seq: JsValue) -> JsValue {
    let v = js_seq_to_vec(&seq);
    if v.is_empty() || v[0] == 0 || v.iter().any(|&x| x < 0) {
        return empty_mountain_result();
    }
    let (mountain, rows) = build_1y_mountain_with_rows(&v);
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &JsValue::from("layers"), &wy_mountain_to_js(&mountain)).ok();
    js_sys::Reflect::set(&obj, &JsValue::from("rows"), &wy_rows_to_js(&rows)).ok();
    obj.into()
}

#[wasm_bindgen(js_name = "expand1Y")]
pub fn expand_1y_js(seq: JsValue, fs: i32) -> JsValue {
    vec_to_js_seq(&expand_1y(&js_seq_to_vec(&seq), fs))
}

#[wasm_bindgen(js_name = "expandWY")]
pub fn expand_wy_js(seq: JsValue, fs: i32) -> JsValue {
    vec_to_js_seq(&expand_wy_seq(&js_seq_to_vec(&seq), fs))
}

#[wasm_bindgen(js_name = "oneYToDBMS")]
pub fn one_y_to_dbms_js(seq: JsValue) -> JsValue {
    matrix_to_js(&one_y_to_dbms(&js_seq_to_vec(&seq)))
}

#[wasm_bindgen(js_name = "dbmsToString")]
pub fn dbms_to_string_js(dbms: JsValue) -> String {
    dbms_to_string(&js_to_matrix_no_pad(&dbms))
}

#[wasm_bindgen(js_name = "dbmsToBMS")]
pub fn dbms_to_bms_js(dbms: JsValue) -> JsValue {
    matrix_to_js(&dbms_to_bms(&js_to_matrix_no_pad(&dbms)))
}

/// Helper: create a progress callback from a JS function.
/// Used by the worker to call bocfToBMS.
#[wasm_bindgen]
pub fn bocf_to_bms_with_progress(input: &str, cb: &js_sys::Function) -> String {
    match bocf_to_bms(input, &mut |s: &str| {
        let _ = cb.call1(&JsValue::null(), &JsValue::from(s));
    }) {
        Ok(r) => r,
        Err(e) => format!("!{}", e),
    }
}
#[wasm_bindgen(js_name = "expandUPMS")]
pub fn expand_upms_js(matrix: JsValue, fs: i32) -> JsValue {
    matrix_to_js(&bms_core::upms::expand_upms(&js_to_matrix_no_pad(&matrix), fs))
}

#[wasm_bindgen(js_name = "isLegalUPMSMatrix")]
pub fn is_legal_upms_matrix_js(matrix: JsValue) -> bool {
    bms_core::upms::is_legal_upms_matrix(&js_to_matrix_no_pad(&matrix))
}

#[wasm_bindgen(js_name = "upmsToBMS")]
pub fn upms_to_bms_js(matrix: JsValue) -> Result<JsValue, JsValue> {
    match bms_core::upms::upms_to_bms(&js_to_matrix_no_pad(&matrix)) {
        Ok(m) => Ok(matrix_to_js(&m)),
        Err(e) => Err(JsValue::from(&js_sys::Error::new(&e))),
    }
}

#[wasm_bindgen(js_name = "bmsToUPMS")]
pub fn bms_to_upms_js(matrix: JsValue) -> Result<JsValue, JsValue> {
    match bms_core::upms::bms_to_upms(&js_to_matrix_no_pad(&matrix)) {
        Ok(m) => Ok(matrix_to_js(&m)),
        Err(e) => Err(JsValue::from(&js_sys::Error::new(&e))),
    }
}

#[wasm_bindgen(js_name = "parseUPMS")]
pub fn parse_upms_js(input: &str) -> Result<JsValue, JsValue> {
    match bms_core::upms::parse_upms(input) {
        Ok(m) => Ok(matrix_to_js(&m)),
        Err(e) => Err(JsValue::from(&js_sys::Error::new(&e))),
    }
}

#[wasm_bindgen(js_name = "formatUPMS")]
pub fn format_upms_js(matrix: JsValue) -> String {
    bms_core::upms::format_upms(&js_to_matrix_no_pad(&matrix))
}
