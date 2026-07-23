#ifndef BMS_PARSER_H
#define BMS_PARSER_H

#include "ordinal.h"
#include <memory>
#include <string>

// ---- AST ----

struct ASTNode;
using ASTPtr = std::shared_ptr<ASTNode>;

struct ASTNode {
  enum Type { Num, W, Omega, Psi, Add, Mul, Pow };
  Type type;
  int numValue = 0;   // for Num
  ASTPtr sub, arg;    // for Omega (sub), Psi (sub, arg)
  ASTPtr left, right; // for Add, Mul, Pow
};

// ---- Public API ----

ASTPtr parseBOCF(const std::string &input);
std::string printAST(const ASTPtr &node, const std::string &indent = "");
TermPtr evalAST(const ASTPtr &node);
std::string bocfToBMS(const std::string &input);

#endif
