#include "ordinal.h"
#include "parser.h"
#include "wy.h"
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

extern std::string g_errorMsg;

// ── Progress callback (used by bocfToBMS) ──
extern "C" void reportBMSProgress(const char *s) {
  fprintf(stderr, "\r  iter %s", s);
  fflush(stderr);
}

// ── Matrix helpers ──

static Matrix parseMatrix(const std::string &s) {
  Matrix M;
  size_t i = 0;
  while (i < s.size()) {
    if (s[i] != '(')
      break;
    size_t end = s.find(')', i);
    if (end == std::string::npos)
      break;
    std::string col = s.substr(i + 1, end - i - 1);
    MatrixRow row;
    size_t pos = 0;
    while (pos < col.size()) {
      size_t comma = col.find(',', pos);
      if (comma == std::string::npos)
        comma = col.size();
      row.push_back(std::stoi(col.substr(pos, comma - pos)));
      pos = comma + 1;
    }
    M.push_back(std::move(row));
    i = end + 1;
  }
  return M;
}

static std::string formatMatrix(const Matrix &M) {
  std::string s;
  for (auto &col : M) {
    s += "(";
    for (size_t i = 0; i < col.size(); i++) {
      if (i > 0)
        s += ",";
      s += std::to_string(col[i]);
    }
    s += ")";
  }
  return s;
}

static std::vector<int> parse0Y(const std::string &s) {
  std::vector<int> seq;
  size_t pos = 0;
  while (pos < s.size()) {
    size_t comma = s.find(',', pos);
    if (comma == std::string::npos)
      comma = s.size();
    seq.push_back(std::stoi(s.substr(pos, comma - pos)));
    pos = comma + 1;
  }
  return seq;
}

// ── Analysis helpers ──

static bool g_latexMode = false;

static bool isFlag(const std::string &arg, const char *flag) { return arg == flag; }

/// Scan args for --latex flag and set g_latexMode, return remaining arg count
static int scanFlags(int argc, char **argv) {
  for (int i = 0; i < argc; i++) {
    if (isFlag(argv[i], "--latex"))
      g_latexMode = true;
  }
  return argc;
}

enum OutputType {
  OUT_ALL,
  OUT_BOCF,
  OUT_BMS,
  OUT_VEBLEN,
  OUT_0Y,
  OUT_TRIANGULAR,
};

static OutputType parseOutputType(const std::string &s) {
  if (s == "bocf" || s == "ocf" || s == "ordinal")
    return OUT_BOCF;
  if (s == "bms")
    return OUT_BMS;
  if (s == "veblen" || s == "φ" || s == "phi")
    return OUT_VEBLEN;
  if (s == "0y" || s == "0-y" || s == "seq")
    return OUT_0Y;
  if (s == "triangular")
    return OUT_TRIANGULAR;
  return OUT_ALL;
}

/// Strip common LaTeX commands for plain terminal output
static std::string stripLatex(const std::string &s) {
  std::string r;
  size_t i = 0;
  while (i < s.size()) {
    if (s[i] == '\\') {
      // LaTeX command
      size_t j = i + 1;
      while (j < s.size() && std::isalpha(s[j]))
        j++;
      std::string cmd = s.substr(i + 1, j - i - 1);
      if (cmd == "psi")
        r += "ψ";
      else if (cmd == "Omega")
        r += "Ω";
      else if (cmd == "omega")
        r += "ω";
      else if (cmd == "varepsilon")
        r += "ε";
      else if (cmd == "zeta")
        r += "ζ";
      else if (cmd == "eta")
        r += "η";
      else if (cmd == "Gamma")
        r += "Γ";
      else if (cmd == "varphi")
        r += "φ";
      else if (cmd == "left" || cmd == "right")
        ; // skip
      else
        r += cmd; // fallback: emit command name without backslash
      // skip the opening brace of the argument (if any)
      while (j < s.size() && s[j] == '{')
        j++;
      i = j;
    } else {
      r += s[i];
      i++;
    }
  }
  return r;
}

static void printBocf(TermPtr ordinal) {
  std::string s = termToString(ordinal, false);
  if (!g_latexMode)
    s = stripLatex(s);
  printf("BOCF: %s\n", s.c_str());
}

static void printVeblen(TermPtr ordinal) {
  std::string v = g_latexMode ? termToVeblen(ordinal) : termToVeblenPlain(ordinal);
  if (!v.empty()) {
    if (!g_latexMode)
      v = stripLatex(v);
    printf("Veblen: %s\n", v.c_str());
  }
}

static void print0Y(const Matrix &M) {
  std::string s = bmsTo0YSequence(M);
  if (!s.empty())
    printf("0-Y: %s\n", s.c_str());
}

static void printBMS(const Matrix &M) {
  printf("BMS: %s\n", formatMatrix(M).c_str());
}

// ── Commands ──

static void printTriBMS(const Matrix &M) {
  Matrix tri = bmsToTriangular(M);
  if (!tri.empty())
    printf("Triangular BMS: %s\n", formatMatrix(tri).c_str());
}

static int cmdBMS(int argc, char **argv) {
  // argv[0]=="bms", argv[1]=input, rest are flags
  scanFlags(argc, argv);
  const std::string &input = argv[0];
  Matrix M = parseMatrix(input);
  if (M.empty()) {
    fprintf(stderr, "Error: could not parse matrix\n");
    return 1;
  }

  // Auto-detect triangular BMS: starts with (0,0,0)(1,0,0)(2,1,0) (extra rows zero-padded)
  auto col0 = M.size() > 0 ? M[0] : MatrixRow();
  auto col1 = M.size() > 1 ? M[1] : MatrixRow();
  auto col2 = M.size() > 2 ? M[2] : MatrixRow();
  auto isZero = [](const MatrixRow &c, int upTo) {
    for (int i = 1; i <= upTo && i < (int)c.size(); i++)
      if (c[i] != 0) return false;
    return true;
  };
  bool isTriangularInput = (M.size() >= 3 &&
    col0.size() >= 1 && col0[0] == 0 && isZero(col0, 2) &&
    col1.size() >= 1 && col1[0] == 1 && isZero(col1, 2) &&
    col2.size() >= 2 && col2[0] == 2 && col2[1] == 1 && (col2.size() <= 2 || col2[2] == 0));

  // Check for explicit --triangular flag
  for (int i = 1; i < argc; i++) {
    if (isFlag(argv[i], "--triangular")) {
      isTriangularInput = true;
      break;
    }
  }

  // Triangular conversion now supports any row count (ancestor-based algorithm).
  size_t maxRowCount = 0;
  for (auto &c : M) if (c.size() > maxRowCount) maxRowCount = c.size();

  Matrix triForm; // saved for OUT_ALL display
  if (isTriangularInput) {
    triForm = M;
    M = triangularToBMS(M);
  }

  // Check for --expand flag
  for (int i = 1; i < argc; i++) {
    if (isFlag(argv[i], "--expand")) {
      int fs = (i + 1 < argc) ? std::stoi(argv[i + 1]) : 1;
      Matrix result = expandBMS(M, fs);
      printf("%s\n", formatMatrix(result).c_str());
      return 0;
    }
  }

  OutputType out = OUT_ALL;
  for (int i = 1; i < argc; i++) {
    if (isFlag(argv[i], "--to") && i + 1 < argc) {
      out = parseOutputType(argv[i + 1]);
      break;
    }
  }

  if (out == OUT_TRIANGULAR) {
    // If input was already triangular, just print original; otherwise convert
    Matrix t = isTriangularInput ? triForm : bmsToTriangular(M);
    printf("%s\n", formatMatrix(t).c_str());
    return 0;
  }

  if (isGteEBO(M)) {
    const char *label = isEqEBO(M) ? "\\psi(I)" : ">\\psi(I)";
    switch (out) {
    case OUT_ALL:
      if (!triForm.empty()) printBMS(M);
      printf("BOCF: %s\n", label);
      print0Y(M);
      if (triForm.empty()) {
        triForm = bmsToTriangular(M);
        printf("Triangular BMS: %s\n", formatMatrix(triForm).c_str());
      }
      break;
    case OUT_BOCF:
      printf("%s\n", label);
      break;
    case OUT_0Y:
      print0Y(M);
      break;
    case OUT_BMS:
      printBMS(M);
      break;
    case OUT_TRIANGULAR:
      break;
    default:
      break;
    }
    return 0;
  }

  TermPtr ordinal = BMSToBocf(M);

  switch (out) {
  case OUT_ALL:
    if (!triForm.empty()) printBMS(M);
    printBocf(ordinal);
    printVeblen(ordinal);
    print0Y(M);
    if (triForm.empty()) {
      triForm = bmsToTriangular(M);
      printf("Triangular BMS: %s\n", formatMatrix(triForm).c_str());
    }
    break;
  case OUT_BOCF:
    printBocf(ordinal);
    break;
  case OUT_BMS:
    printBMS(M);
    break;
  case OUT_VEBLEN:
    printVeblen(ordinal);
    break;
  case OUT_0Y:
    print0Y(M);
    break;
  case OUT_TRIANGULAR:
    break;
  }
  return 0;
}

static int cmdBOCF(int argc, char **argv) {
  const std::string &input = argv[0];
  scanFlags(argc, argv);

  // Check for --expand flag first
  for (int i = 1; i < argc; i++) {
    if (isFlag(argv[i], "--expand") && i + 1 < argc) {
      int n = std::stoi(argv[i + 1]);
      g_errorMsg.clear();
      ASTPtr ast = parseBOCF(input);
      if (!g_errorMsg.empty()) {
        fprintf(stderr, "Parse error: %s\n", g_errorMsg.c_str());
        return 1;
      }
      TermPtr val = evalAST(ast);
      if (!g_errorMsg.empty()) {
        fprintf(stderr, "Eval error: %s\n", g_errorMsg.c_str());
        return 1;
      }
      val = standardForm(val);
      TermPtr fs = fundamentalSequence(val, n);
      std::string s = termToString(fs, false);
      if (!g_latexMode) s = stripLatex(s);
      printf("%s\n", s.c_str());
      return 0;
    }
  }

  // Normal BOCF→BMS path
  fprintf(stderr, "Converting BOCF to BMS...\n");
  std::string result = bocfToBMS(input);
  fprintf(stderr, "\n");
  if (!result.empty() && result[0] == '!') {
    fprintf(stderr, "Error: %s\n", result.c_str() + 1);
    return 1;
  }

  Matrix M = parseMatrix(result);
  if (M.empty()) {
    printf("%s\n", result.c_str());
    return 0;
  }

  // Check for --to flag
  OutputType out = OUT_ALL;
  for (int i = 1; i < argc; i++) {
    if (isFlag(argv[i], "--to") && i + 1 < argc) {
      out = parseOutputType(argv[i + 1]);
      break;
    }
  }

  if (out == OUT_BMS) {
    printf("%s\n", result.c_str());
    return 0;
  }

  TermPtr ordinal = BMSToBocf(M);

  switch (out) {
  case OUT_ALL:
    // BOCF input: by default show BMS, Veblen, 0-Y (not BOCF — user already knows it)
    printBMS(M);
    printTriBMS(M);
    printVeblen(ordinal);
    print0Y(M);
    break;
  case OUT_BOCF:
    printBocf(ordinal);
    break;
  case OUT_VEBLEN:
    printVeblen(ordinal);
    break;
  case OUT_0Y:
    print0Y(M);
    break;
  default:
    printf("%s\n", result.c_str());
    break;
  }
  return 0;
}

static int cmd0Y(int argc, char **argv) {
  const std::string &input = argv[0];
  scanFlags(argc, argv);

  // Check for --expand flag first
  for (int i = 1; i < argc; i++) {
    if (isFlag(argv[i], "--expand") && i + 1 < argc) {
      int n = std::stoi(argv[i + 1]);
      std::vector<int> seq = parse0Y(input);
      std::vector<int> expanded = zeroYExpand(seq, n);
      for (size_t j = 0; j < expanded.size(); j++) {
        if (j > 0) printf(",");
        printf("%d", expanded[j]);
      }
      printf("\n");
      return 0;
    }
  }

  std::vector<int> seq = parse0Y(input);
  Matrix M = zeroYToBMS(seq);

  // Check for --to flag
  OutputType out = OUT_ALL;
  for (int i = 1; i < argc; i++) {
    if (isFlag(argv[i], "--to") && i + 1 < argc) {
      out = parseOutputType(argv[i + 1]);
      break;
    }
  }

  switch (out) {
  case OUT_ALL:
    // 0-Y input: show BMS, BOCF, Veblen (not 0-Y — user already knows it)
    printBMS(M);
    printTriBMS(M);
    if (isGteEBO(M)) {
      printf("BOCF: %s\n", isEqEBO(M) ? "\\psi(I)" : ">\\psi(I)");
    } else {
      TermPtr ordinal = BMSToBocf(M);
      printBocf(ordinal);
      printVeblen(ordinal);
    }
    break;
  case OUT_BMS:
    printBMS(M);
    break;
  case OUT_BOCF:
    if (isGteEBO(M)) {
      printf("%s\n", isEqEBO(M) ? "\\psi(I)" : ">\\psi(I)");
    } else {
      TermPtr ordinal = BMSToBocf(M);
      printBocf(ordinal);
    }
    break;
  case OUT_VEBLEN:
    if (isGteEBO(M))
      break;
    {
      TermPtr ordinal = BMSToBocf(M);
      printVeblen(ordinal);
    }
    break;
  default:
    break;
  }
  return 0;
}

static int cmd1Y(int argc, char **argv) {
  if (argc < 1) {
    fprintf(stderr, "Usage: analyzer-cli 1y <sequence> [--expand <n>]\n");
    return 1;
  }
  const std::string &input = argv[0];
  scanFlags(argc, argv);
  int fs = -1;
  for (int i = 1; i < argc; i++) {
    if (isFlag(argv[i], "--expand") && i + 1 < argc) {
      fs = std::stoi(argv[i + 1]);
      break;
    }
  }
  if (fs < 0) {
    fprintf(stderr, "Error: --expand is required\n");
    return 1;
  }
  std::vector<int> seq = parse0Y(input);
  std::vector<int> expanded = expand1Y(seq, fs);
  for (size_t j = 0; j < expanded.size(); j++) {
    if (j > 0) printf(",");
    printf("%d", expanded[j]);
  }
  printf("\n");
  return 0;
}

static int cmdWY(int argc, char **argv) {
  if (argc < 1) {
    fprintf(stderr, "Usage: analyzer-cli wy <sequence> [--expand <n>]\n");
    return 1;
  }
  const std::string &input = argv[0];
  scanFlags(argc, argv);
  int fs = -1;
  for (int i = 1; i < argc; i++) {
    if (isFlag(argv[i], "--expand") && i + 1 < argc) {
      fs = std::stoi(argv[i + 1]);
      break;
    }
  }
  if (fs < 0) {
    fprintf(stderr, "Error: --expand is required\n");
    return 1;
  }
  std::vector<int> seq = parse0Y(input);
  std::vector<int> expanded = expandWY(seq, fs);
  for (size_t j = 0; j < expanded.size(); j++) {
    if (j > 0) printf(",");
    printf("%d", expanded[j]);
  }
  printf("\n");
  return 0;
}

static int cmdFS(int argc, char **argv) {  scanFlags(argc, argv);  if (argc < 2) {    fprintf(stderr, "Usage: analyzer-cli fs <expr> <n>\n");    return 1;  }  const std::string &input = argv[0];  int n = std::stoi(argv[1]);  g_errorMsg.clear();  ASTPtr ast = parseBOCF(input);  if (!g_errorMsg.empty()) {    fprintf(stderr, "Parse error: %s\n", g_errorMsg.c_str());    return 1;  }  TermPtr val = evalAST(ast);  if (!g_errorMsg.empty()) {    fprintf(stderr, "Eval error: %s\n", g_errorMsg.c_str());    return 1;  }  val = standardForm(val);  TermPtr fs = fundamentalSequence(val, n);  std::string s = termToString(fs, false);  if (!g_latexMode) s = stripLatex(s);  printf("%s\n", s.c_str());  return 0;}

static void printUsage() {
  printf(
    "Usage:\n"
    "  analyzer-cli bms <matrix> [--to <fmt>] [--expand <fs>] [--triangular]\n"
    "  analyzer-cli bocf <expr> [--to <fmt>] [--expand <n>]\n"
    "  analyzer-cli 0y <sequence> [--to <fmt>] [--expand <n>]\n"
    "  analyzer-cli 1y <sequence> --expand <n>\n"
    "  analyzer-cli wy <sequence> --expand <n>\n"
    "  analyzer-cli fs <expr> <n>   Fundamental sequence of a BOCF expression\n"
    "\n"
    "Commands:\n"
    "  bms   Analyze or expand a BMS matrix (or triangular BMS with --triangular)\n"
    "  bocf  Convert BOCF expression to BMS (or expand with --expand)\n"
    "  0y    Convert 0-Y sequence to BMS (or expand with --expand)\n"
    "  1y    Expand a 1-Y sequence\n"
    "  wy    Expand a ω-Y sequence\n"
    "  fs    Compute the nth term of an ordinal\047s fundamental sequence\n"
    "\n"
    "Flags:\n"
    "  --to <fmt>     Output a specific notation (bocf, bms, veblen, 0y, triangular)\n"
    "  --expand <n>   Expand BMS matrix or compute FS of BOCF (n = step)\n"
    "  --triangular   Treat input matrix as triangular BMS (convert to standard before analysis)\n"
    "  --latex        Output LaTeX formatting for Veblen\n"
    "\n"
    "Formats:\n"
    "  Matrix:  (0,0,0)(1,1,1)(2,2,0)\n"
    "  0-Y:     1,2,3,4\n"
    "  BOCF:    p(w)  (p=ψ, w=ω, W=Ω)\n");

}

int main(int argc, char **argv) {
  if (argc < 2) {
    printUsage();
    return 1;
  }

  std::string cmd = argv[1];

  if (cmd == "-h" || cmd == "--help") {
    printUsage();
    return 0;
  }

  // Remaining args after the command word
  int cmdArgc = argc - 2;
  char **cmdArgv = argv + 2;

  if (cmdArgc < 1) {
    printUsage();
    return 1;
  }

  if (cmd == "bms") {
    return cmdBMS(cmdArgc, cmdArgv);
  } else if (cmd == "bocf") {
    return cmdBOCF(cmdArgc, cmdArgv);
  } else if (cmd == "0y") {
    return cmd0Y(cmdArgc, cmdArgv);
  } else if (cmd == "1y") {
    return cmd1Y(cmdArgc, cmdArgv);
  } else if (cmd == "wy") {
    return cmdWY(cmdArgc, cmdArgv);
  } else if (cmd == "fs") {
    return cmdFS(cmdArgc, cmdArgv);
  } else {
    fprintf(stderr, "Unknown command: %s\n\n", cmd.c_str());
    printUsage();
    return 1;
  }
}
