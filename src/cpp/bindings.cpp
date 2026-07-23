#include "ordinal.h"
#include "parser.h"
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

EMSCRIPTEN_BINDINGS(bms_core) {
  function("bmsAnalyze", &bmsAnalyze);
  function("matrixLexOrder", &matrixLexOrderJS);
  function("decomposePower", &decomposePowerToJS);
  function("computeT", &computeTToJS);
  function("zeroYToBMS", &zeroYToBMSJS);
  function("parseAndEvalBOCF", &parseAndEvalBOCF);
  function("expandBMS", &expandBMSJS);
  function("bmsTo0YSequence", &bmsTo0YSequenceJS);
  function("subscriptDepth", &subscriptDepthJS);
  function("termToVeblen", &termToVeblenJS);
  function("bocfToBMS", &bocfToBMSJS);
}
