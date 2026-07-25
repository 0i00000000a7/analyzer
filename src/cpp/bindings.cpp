#include "1y.h"
#include "ordinal.h"
#include "parser.h"
#include "wy.h"
#include <emscripten.h>
#include <emscripten/bind.h>
#include <emscripten/val.h>
#include <sstream>

using namespace emscripten;

// Error string (defined in parser.cpp)
extern std::string g_errorMsg;

static Matrix jsArrayToMatrix(const val &jsArr) {
  Matrix M;
  int len = jsArr["length"].as<int>();
  for (int i = 0; i < len; i++) {
    val row = jsArr[i];
    int rowLen = row["length"].as<int>();
    MatrixRow r;
    for (int j = 0; j < rowLen; j++) {
      r.push_back(row[j].as<int>());
    }
    while ((int)r.size() < 3)
      r.push_back(0);
    M.push_back(std::move(r));
  }
  return M;
}

static val termToJS(TermPtr t) {
  if (isZero(t)) {
    return val::array();
  }
  val arr = val::array();
  arr.call<void>("push", termToJS(t->a));
  arr.call<void>("push", termToJS(t->b));
  arr.call<void>("push", termToJS(t->c));
  return arr;
}

static TermPtr termFromJS(const val &js) {
  if (js["length"].as<int>() == 0)
    return ZERO();
  TermPtr a = termFromJS(js[0]);
  TermPtr b = termFromJS(js[1]);
  TermPtr c = termFromJS(js[2]);
  return T(a, b, c);
}

static val decomposePowerToJS(const val &jsTerm) {
  TermPtr t = termFromJS(jsTerm);
  if (isZero(t)) {
    val r = val::array();
    r.call<void>("push", val::array());
    r.call<void>("push", val::array());
    return r;
  }
  auto [first, second] = debugDecomposePower(t);
  val r = val::array();
  r.call<void>("push", termToJS(first));
  r.call<void>("push", termToJS(second));
  return r;
}

static val computeTToJS(const val &jsTerm) {
  TermPtr t = termFromJS(jsTerm);
  return termToJS(debugComputeT(t));
}

static std::string termToStr(TermPtr t) { return termToString(t, false); }

val bmsAnalyze(const val &jsMatrix) {
  Matrix M = jsArrayToMatrix(jsMatrix);

  if (isGteEBO(M)) {
    val result = val::object();
    result.set("gteEBO", true);
    if (isEqEBO(M)) {
      result.set("ordinal", std::string("\\psi(I)"));
    } else {
      result.set("ordinal", std::string(">\\psi(I)"));
    }
    return result;
  }

  TermPtr ordinal = BMSToBocf(M);
  TermPtr nsForm = ordinal;

  val result = val::object();
  result.set("gteEBO", false);
  result.set("ordinal", termToStr(ordinal));
  result.set("ordinalJS", termToJS(ordinal));
  result.set("veblen", termToVeblen(ordinal));
  result.set("veblenPlain", termToVeblenPlain(ordinal));
  result.set("veblenMatrix", termToVeblenMatrix(ordinal));
  result.set("veblenMatrixPlain", termToVeblenMatrixPlain(ordinal));
  result.set("nsForm", termToStr(nsForm));
  result.set("isStandard", eq(nsForm, ordinal));

  return result;
}

int matrixLexOrderJS(const val &jsA, const val &jsB) {
  Matrix a = jsArrayToMatrix(jsA);
  Matrix b = jsArrayToMatrix(jsB);
  return matrixLexOrder(a, b);
}

val zeroYToBMSJS(const val &jsSeq) {
  int len = jsSeq["length"].as<int>();
  std::vector<int> seq;
  for (int i = 0; i < len; i++) {
    seq.push_back(jsSeq[i].as<int>());
  }

  Matrix M = zeroYToBMS(seq);

  val result = val::array();
  for (size_t i = 0; i < M.size(); i++) {
    val col = val::array();
    for (size_t j = 0; j < M[i].size(); j++) {
      col.call<void>("push", M[i][j]);
    }
    result.call<void>("push", col);
  }
  return result;
}

val zeroYExpandJS(const val &jsSeq, int n) {
  int len = jsSeq["length"].as<int>();
  std::vector<int> seq;
  for (int i = 0; i < len; i++) {
    seq.push_back(jsSeq[i].as<int>());
  }
  auto result = zeroYExpand(seq, n);
  val jsResult = val::array();
  for (auto v : result) {
    jsResult.call<void>("push", v);
  }
  return jsResult;
}

val parseAndEvalBOCF(const std::string &input) {
  val result = val::object();
  g_errorMsg.clear();
  ASTPtr ast = parseBOCF(input);
  if (!g_errorMsg.empty()) {
    result.set("ast", std::string(""));
    result.set("ordinal", std::string(""));
    result.set("ordinalJS", val::array());
    result.set("error", g_errorMsg);
  } else {
    result.set("ast", printAST(ast));
    TermPtr ordinal = evalAST(ast);
    if (!g_errorMsg.empty()) {
      result.set("error", g_errorMsg);
    } else {
      result.set("ordinal", termToString(ordinal, true));
      result.set("ordinalJS", termToJS(ordinal));
      result.set("error", std::string(""));
    }
  }
  return result;
}

val expandBMSJS(const val &jsMatrix, int fs) {
  Matrix M = jsArrayToMatrix(jsMatrix);
  Matrix result = expandBMS(M, fs);
  val jsResult = val::array();
  for (size_t i = 0; i < result.size(); i++) {
    val col = val::array();
    for (size_t j = 0; j < result[i].size(); j++) {
      col.call<void>("push", result[i][j]);
    }
    jsResult.call<void>("push", col);
  }
  return jsResult;
}

std::string bmsTo0YSequenceJS(const val &jsMatrix) {
  Matrix M = jsArrayToMatrix(jsMatrix);
  return bmsTo0YSequence(M);
}

int subscriptDepthJS(const val &jsTerm) { return subscriptDepth(termFromJS(jsTerm)); }

val fundamentalSequenceJS(const val &jsTerm, int n) {
  TermPtr t = termFromJS(jsTerm);
  TermPtr result = fundamentalSequence(t, n);
  val r = val::object();
  r.set("term", termToString(result, false));
  r.set("termJS", termToJS(result));
  return r;
}

val cofinalityJS(const val &jsTerm) {
  TermPtr t = termFromJS(jsTerm);
  TermPtr result = cofinality(t);
  val r = val::object();
  r.set("term", termToString(result, false));
  r.set("termJS", termToJS(result));
  return r;
}

val termToVeblenJS(const val &jsTerm) {
  TermPtr t = termFromJS(jsTerm);
  val result = val::object();
  result.set("veblen", termToVeblen(t));
  result.set("veblenPlain", termToVeblenPlain(t));
  result.set("veblenMatrix", termToVeblenMatrix(t));
  result.set("veblenMatrixPlain", termToVeblenMatrixPlain(t));
  return result;
}

static val g_progressCB;

extern "C" void reportBMSProgress(const char *s) {
  if (!g_progressCB.isUndefined()) {
    g_progressCB(std::string(s));
  }
}

val bocfToBMSJS(const std::string &input, val progressCB) {
  val result = val::object();
  g_progressCB = progressCB;
  std::string bms = bocfToBMS(input);
  g_progressCB = val::undefined();
  if (!bms.empty() && bms[0] == '!') {
    result.set("result", std::string(""));
    result.set("error", bms.substr(1));
  } else {
    result.set("result", bms);
    result.set("error", std::string(""));
  }
  return result;
}

val triangularToBMSJS(const val &jsMatrix) {
  Matrix M = jsArrayToMatrix(jsMatrix);
  Matrix result = triangularToBMS(M);
  val jsResult = val::array();
  for (size_t i = 0; i < result.size(); i++) {
    val col = val::array();
    for (size_t j = 0; j < result[i].size(); j++) {
      col.call<void>("push", result[i][j]);
    }
    jsResult.call<void>("push", col);
  }
  return jsResult;
}

val bmsToTriangularJS(const val &jsMatrix) {
  Matrix M = jsArrayToMatrix(jsMatrix);
  Matrix result = bmsToTriangular(M);
  val jsResult = val::array();
  for (size_t i = 0; i < result.size(); i++) {
    val col = val::array();
    for (size_t j = 0; j < result[i].size(); j++) {
      col.call<void>("push", result[i][j]);
    }
    jsResult.call<void>("push", col);
  }
  return jsResult;
}

val buildMountainJS(const val &jsSeq) {
  int len = jsSeq["length"].as<int>();
  std::vector<int> seq;
  for (int i = 0; i < len; i++) {
    seq.push_back(jsSeq[i].as<int>());
  }
  Mountain m = buildMountain(seq);
  val jsLayers = val::array();
  for (auto &layer : m) {
    val jsLayer = val::array();
    for (auto &p : layer) {
      val jsNode = val::object();
      jsNode.set("value", p.first);
      jsNode.set("parent", p.second);
      jsLayer.call<void>("push", jsNode);
    }
    jsLayers.call<void>("push", jsLayer);
  }
  return jsLayers;
}

// ── 1-Y / ω-Y JS wrappers (forward declarations) ──
val expand1YJS(const val &jsSeq, int fs);
val expandWYJS(const val &jsSeq, int fs);
val buildWYMountainJS(const val &jsSeq, int n, bool consistent);
val build1YMountainJS(const val &jsSeq);
// DBMS
val oneYToDBMSJS(const val &jsSeq);
std::string dbmsToStringJS(const val &jsDBMS);
val dbmsToBMSJS(const val &jsDBMS);

EMSCRIPTEN_BINDINGS(bms_core) {
  function("bmsAnalyze", &bmsAnalyze);
  function("matrixLexOrder", &matrixLexOrderJS);
  function("decomposePower", &decomposePowerToJS);
  function("computeT", &computeTToJS);
  function("zeroYToBMS", &zeroYToBMSJS);
  function("zeroYExpand", &zeroYExpandJS);
  function("parseAndEvalBOCF", &parseAndEvalBOCF);
  function("expandBMS", &expandBMSJS);
  function("bmsTo0YSequence", &bmsTo0YSequenceJS);
  function("subscriptDepth", &subscriptDepthJS);
  function("termToVeblen", &termToVeblenJS);
  function("bocfToBMS", &bocfToBMSJS);
  function("fundamentalSequence", &fundamentalSequenceJS);
  function("cofinality", &cofinalityJS);
  function("triangularToBMS", &triangularToBMSJS);
  function("bmsToTriangular", &bmsToTriangularJS);
  function("buildMountain", &buildMountainJS);
  function("buildWYMountain", &buildWYMountainJS);
  function("build1YMountain", &build1YMountainJS);
  function("expand1Y", &expand1YJS);
  function("expandWY", &expandWYJS);
  function("oneYToDBMS", &oneYToDBMSJS);
  function("dbmsToString", &dbmsToStringJS);
  function("dbmsToBMS", &dbmsToBMSJS);
}

// ── 1-Y / ω-Y JS wrappers ──

static std::vector<int> seqFromVal(const val &jsSeq) {
  int len = jsSeq["length"].as<int>();
  std::vector<int> seq;
  for (int i = 0; i < len; i++)
    seq.push_back(jsSeq[i].as<int>());
  return seq;
}

static val seqToVal(const std::vector<int> &seq) {
  val js = val::array();
  for (int v : seq)
    js.call<void>("push", v);
  return js;
}

val expand1YJS(const val &jsSeq, int fs) {
  auto seq = seqFromVal(jsSeq);
  if (seq.empty() || seq[0] == 0)
    return seqToVal(seq);
  for (int v : seq)
    if (v < 0)
      return seqToVal(seq);
  auto result = expand1Y(seq, fs);
  return seqToVal(result);
}

val expandWYJS(const val &jsSeq, int fs) {
  auto seq = seqFromVal(jsSeq);
  if (seq.empty() || seq[0] == 0)
    return seqToVal(seq);
  for (int v : seq)
    if (v < 0)
      return seqToVal(seq);
  auto result = expandWY(seq, fs);
  return seqToVal(result);
}

static val emptyMountainResult() {
  val r = val::object();
  r.set("layers", val::array());
  r.set("rows", val::array());
  return r;
}

val buildWYMountainJS(const val &jsSeq, int n, bool consistent = false) {
  int len = jsSeq["length"].as<int>();
  std::vector<int> seq;
  for (int i = 0; i < len; i++)
    seq.push_back(jsSeq[i].as<int>());
  if (seq.empty() || seq[0] == 0)
    return emptyMountainResult();
  for (int v : seq)
    if (v < 0)
      return emptyMountainResult();
  auto [m, rowLabels] = buildWYMountainWithRows(seq, n, consistent);

  val jsLayers = val::array();
  for (auto &layer : m) {
    val jsLayer = val::array();
    for (int c = 0; c < (int)layer.size(); c++) {
      val jsNode = val::object();
      jsNode.set("value", layer[c].first);
      jsNode.set("parent", layer[c].second);
      // parent column: col - parent (parent is distance)
      jsNode.set("parentCol", layer[c].second > 0 ? c - layer[c].second : -1);
      jsLayer.call<void>("push", jsNode);
    }
    jsLayers.call<void>("push", jsLayer);
  }

  val jsRows = val::array();
  for (auto &ord : rowLabels) {
    val jsOrd = val::array();
    // Pass raw little-endian: [a0, a1, a2] = a0·ω^0 + a1·ω^1 + a2·ω^2
    for (int v : ord) {
      jsOrd.call<void>("push", v);
    }
    jsRows.call<void>("push", jsOrd);
  }

  val result = val::object();
  result.set("layers", jsLayers);
  result.set("rows", jsRows);
  return result;
}

val build1YMountainJS(const val &jsSeq) {
  int len = jsSeq["length"].as<int>();
  std::vector<int> seq;
  for (int i = 0; i < len; i++)
    seq.push_back(jsSeq[i].as<int>());
  if (seq.empty() || seq[0] == 0)
    return emptyMountainResult();
  for (int v : seq)
    if (v < 0)
      return emptyMountainResult();
  auto [m, rowLabels] = build1YMountainWithRows(seq);

  val jsLayers = val::array();
  for (auto &layer : m) {
    val jsLayer = val::array();
    for (int c = 0; c < (int)layer.size(); c++) {
      val jsNode = val::object();
      jsNode.set("value", layer[c].first);
      jsNode.set("parent", layer[c].second);
      jsNode.set("parentCol", layer[c].second > 0 ? c - layer[c].second : -1);
      jsLayer.call<void>("push", jsNode);
    }
    jsLayers.call<void>("push", jsLayer);
  }

  val jsRows = val::array();
  for (auto &ord : rowLabels) {
    val jsOrd = val::array();
    for (int v : ord)
      jsOrd.call<void>("push", v);
    jsRows.call<void>("push", jsOrd);
  }

  val result = val::object();
  result.set("layers", jsLayers);
  result.set("rows", jsRows);
  return result;
}

// ── DBMS conversion JS wrappers ──

val oneYToDBMSJS(const val &jsSeq) {
  auto seq = seqFromVal(jsSeq);
  if (seq.empty() || seq[0] == 0)
    return val::array();
  for (int v : seq)
    if (v < 0)
      return val::array();
  Matrix result = oneYToDBMS(seq);
  val jsResult = val::array();
  for (auto &col : result) {
    val jsCol = val::array();
    for (int v : col)
      jsCol.call<void>("push", v);
    jsResult.call<void>("push", jsCol);
  }
  return jsResult;
}

std::string dbmsToStringJS(const val &jsDBMS) {
  Matrix M;
  int cols = jsDBMS["length"].as<int>();
  for (int i = 0; i < cols; i++) {
    val jsCol = jsDBMS[i];
    int rowLen = jsCol["length"].as<int>();
    MatrixRow col;
    for (int j = 0; j < rowLen; j++)
      col.push_back(jsCol[j].as<int>());
    M.push_back(std::move(col));
  }
  return dbmsToString(M);
}

val dbmsToBMSJS(const val &jsDBMS) {
  Matrix M;
  int cols = jsDBMS["length"].as<int>();
  for (int i = 0; i < cols; i++) {
    val jsCol = jsDBMS[i];
    int rowLen = jsCol["length"].as<int>();
    MatrixRow col;
    for (int j = 0; j < rowLen; j++)
      col.push_back(jsCol[j].as<int>());
    M.push_back(std::move(col));
  }
  Matrix result = dbmsToBMS(M);
  val jsResult = val::array();
  for (auto &col : result) {
    val jsCol = val::array();
    for (int v : col)
      jsCol.call<void>("push", v);
    jsResult.call<void>("push", jsCol);
  }
  return jsResult;
}
