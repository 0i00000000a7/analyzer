/*
BMS analyzer by Solarzone
由 FiveYearGaoKao, VeryrrDefine 进行批注和修改

修改内容:
1.为提高可读性，对部分变量和函数名进行了修改
2.添加了一些辅助函数
3.添加了部分typescript类型

程序介绍:
此程序分为两部分：序数表示和BMS分析
存储序数的结构类似单向链表
程序用[a,b,c]表示序数ψ_a(b)+c(使用BOCF)
BMS分析的部分暂时没有看懂
*/

/**
 * @typedef {([]|ThirdTerm)} Term
 * @typedef {([Term, Term, Term])} ThirdTerm
 * @typedef {([number, number, number])} MatrixRow
 * @typedef {(MatrixRow[])} Matrix
 */

/**
 * @type {[]}
 */
const ZERO = [];
/**
 * @type {[[],[],[]]}
 */
const ONE = [[], [], []];
/**
 * @type {[[],[[],[],[]],[]]}
 */
const OMEGA = [[], ONE, []];
/**
 * @type {[[[],[],[]],[],[]]}
 */
const OMEGA1 = [ONE, [], []];
/**
 * @type {[[],[[[],[],[]],[],[]],[]]}
 */
const EPSILON0 = [[], OMEGA1, []];

/**
 * 判断一个序数是否为0
 * @param {Term|number} a
 * @returns {a is []}
 */
function isZero(a) {
  return a.length == 0;
}

/**
 * 判断一个序数是否有限
 * @param {Term} a
 */
function isOrdinalFinite(a) {
  return isZero(a) || (isZero(a[0]) && isZero(a[1]));
}

/**
 * 求一个序数由多少个单项相加而成
 * @param {Term} a
 * @returns {number} 单项数
 */
function length1(a) {
  return isZero(a) ? 0 : 1 + length1(a[2]);
}

/**
 * 判断两个序数是否全等
 * 甚至也可以判断两个矩阵列是否全等
 * @param {Term| number} a
 * @param {Term| number} b
 * @returns {boolean} 是否全等
 */
function eq(a, b) {
  if (typeof a == "number") {
    return a == b;
  }
  if (isZero(a) || isZero(b)) {
    return isZero(a) == isZero(b);
  }
  return eq(a[0], b[0]) && eq(a[1], b[1]) && eq(a[2], b[2]);
}

/**
 * 判断a是否小于b
 * @param {Term|number} a
 * @param {Term|number} b
 * @returns {boolean} a是否小于b
 */
function lt(a, b) {
  if (isZero(b)) {
    return false;
  }
  if (isZero(a)) {
    return true;
  }
  if (!eq(a[0], b[0])) {
    return lt(a[0], b[0]);
  }
  if (!eq(a[1], b[1])) {
    return lt(a[1], b[1]);
  }
  return lt(a[2], b[2]);
}

/**
 * 判断a是否大于b
 * @param {Term|number} a
 * @param {Term|number} b
 * @returns {boolean} a是否大于b
 */
function gt(a, b) {
  return !(lt(a, b) || eq(a, b));
}

/**
 * ω^a1+ω^a2+...+ω^an的首项ω^a1
 *
 * 相当于把加法部分换成0
 *
 * 1的首项ω^0,则返回1
 *
 * OMEGA [0, 1, 0]的首项为ω^1，则返回[0,1,0]
 *
 * @param {Term} a
 * @returns {Term}
 */
function firstTerm(a) {
  if (isZero(a)) {
    return [];
  }
  return [a[0], a[1], []];
}

/**
 * ω^a1+ω^a2+...+ω^an的末项ω^an
 * @param {Term} a
 * @returns {Term}
 */
function lastTerm(a) {
  if (isZero(a)) {
    return [];
  }
  if (isZero(a[2])) {
    return a;
  }
  return lastTerm(a[2]);
}

//ψa(b)
function psi(a) {}

/**
 * 序数相加
 * @param {Term} a
 * @param {Term} b
 * @returns {Term}
 */
function add(a, b) {
  if (isZero(a)) {
    return b;
  }
  if (isZero(b)) {
    return a;
  }
  if (lt(firstTerm(a), firstTerm(b))) {
    return b;
  }
  return [a[0], a[1], add(a[2], b)];
}

/**
 * 序数后继
 * @param {Term} a
 * @returns {Term}
 */
function succ(a) {
  return add(a, ONE);
}

/**
 * 序数左减，即a-b为满足b+c=a的序数c(若不存在为0)
 * @param {Term} a
 * @param {Term} b
 * @returns {Term}
 */
function sub(a, b) {
  if (isZero(a)) {
    return [];
  }
  if (isZero(b)) {
    return a;
  }
  if (gt(firstTerm(a), firstTerm(b))) {
    return a;
  }
  return sub(a[2], b[2]);
}

/**
 * 将a分为大于b和小于b两段
 * @param {Term} a
 * @param {Term} b
 * @returns {[Term, Term]}
 */
function separate(a, b) {
  if (isZero(a)) {
    return [[], []];
  }
  if (lt(firstTerm(a), b)) {
    return [[], a];
  }
  return [[a[0], a[1], separate(a[2], b)[0]], separate(a[2], b)[1]];
}

/**
 * 将a的标准式中所有小于ψb(0)的项全部截断
 * @param {Term} a
 * @param {Term} b
 * @returns {Term}
 */
function truncate(a, b) {
  if (isZero(a)) {
    return [];
  }
  if (isZero(truncate(a[2], b)) && lt(firstTerm(a), [b, [], []])) {
    return [];
  }
  return [a[0], a[1], truncate(a[2], b)];
}

/**
 *序数ω^a，自动化为标准式
设a=ψb(p+d)+e,其中b的每一项都大于等于ψb+1(0)
则该函数返回的是ψb(p+{a-ψb(p)})
注意到当d<ψb+1(0)时,ψb(c+d)=ψb(c)*ω^d
分情况讨论：
1.若d=e=0，则ψb(p)=ψb(...+ψb+1(0))是一个ε点，取指数后不变
2.若d=0,e>0，函数返回ψb(p+e)=ψb(p)*ω^e=ω^(ψb(p)+e)
3.若d>0，函数返回ψb(p+a)=ψb(p)*ω^a=ω^(ψb(p)+a)=ω^a
 * @param {Term} a
 * @returns {Term}

*/
function exp(a) {
  if (lt(a, EPSILON0)) {
    return [[], a, []];
  }
  let p = separate(a[1], [succ(a[0]), [], []])[0];
  return [a[0], add(p, sub(a, [a[0], p, []])), []];
}

/**
 *
 * log_ω(a),即满足ω^b<=a的最大序数b
 *
 * @param {Term} a
 * @returns {Term}
 */
function log(a) {
  if (isZero(a)) {
    return [];
  }
  let [p, q] = separate(a[1], [succ(a[0]), [], []]);
  //同上，设a=ψb(p+q)+e,其中b的每一项都大于等于ψb+1(0)
  if (isZero(a[0]) && isZero(p)) {
    //此时a=ψ(q)+e,q<Ω,若为标准式则一定有a<ε0
    if (!lt(a[1], EPSILON0)) {
      //q>=ε0
      if (eq(log(q), q) && isZero(q[2]) && lt(a[1], OMEGA1)) {
        return firstTerm(a);
      }
      //q是ε点,q是ω的幂,q<Ω
    }
    return q;
  }
  let m = add([a[0], p, []], q); //m=ψb(p)+q,ω^m=ψb(p+q)
  if (!lt(a[1], [a[0], [succ(a[0]), [], []], []])) {
    //p+q>=ψb(ψb+1(0))
    if (eq(log(a[1]), a[1]) && isZero(a[2]) && lt(a[1], [succ(a[0]), [], []])) {
      return firstTerm(a);
    }
    //p+q是ε点,a是ω的幂,p+q<ψb+1(0)
  }
  return m;
}

//
/**
 * 找BMS的父项用的
 *
 * 在第0(row)行中，查找相对于第3(n)列（值3）具有更小值的最近前驱列
 * 如矩阵[ [ 0, 0, 0 ], [ 1, 1, 1 ], [ 2, 2, 2 ] ]
 * @param {Matrix} matrix
 * @param {number} findRow
 * @param {number} relativeColumn
 * @returns {number}
 */
function findMatrixParentTerm(matrix, findRow, relativeColumn) {
  if (findRow == -1) {
    return relativeColumn - 1;
  }
  let curColumn = findMatrixParentTerm(matrix, findRow - 1, relativeColumn);
  while (
    curColumn > -1 &&
    matrix[curColumn][findRow] >= matrix[relativeColumn][findRow]
  ) {
    curColumn = findMatrixParentTerm(matrix, findRow - 1, curColumn);
  }
  return curColumn;
}

/**
 * 求第1行第n列的子项
 * 有哪些项的坏根是column
 * @param {Matrix} M
 * @param {number} n
 * @returns {number[]}
 */
function children(M, n) {
  let X = [];
  for (let i = 0; i < M.length; i++) {
    if (findMatrixParentTerm(M, 0, i) == n) {
      X.push(i);
    }
  }
  return X;
}

//第1行第n列“第二行大于1”的子项个数
function D(M, n) {
  let X = 0;
  for (let i = 0; i < M.length; i++) {
    if (findMatrixParentTerm(M, 0, i) == n && M[i][1] > 0) {
      X++;
    }
  }
  return X;
}

/**
 * 查找提升效应项
 * @param {Matrix} M
 * @param {number} n
 * @returns {number} -1为没有找到符合的index
 */
function checkW_wLikeFunction(M, n) {
  if (M[n][1] == 0 || M[n][2] == 1 || n + 1 == M.length) {
    return -1;
  }
  //只接受形如(x,y,0)的非末项
  let m = findMatrixParentTerm(M, 1, n);
  let L = [M[m][0] + 1, M[n][1], M[m][2] + 1];
  if (
    findMatrixParentTerm(M, 1, n) == findMatrixParentTerm(M, 1, n + 1) &&
    eq(M[n + 1], L)
  ) {
    return n + 1;
  }
  let q = n;
  while (q != -1) {
    q = findMatrixParentTerm(M, 0, q);
    if (
      findMatrixParentTerm(M, 1, n) == findMatrixParentTerm(M, 1, q) &&
      eq(M[q], L) &&
      M[n + 1][0] > M[q][0]
    ) {
      return q;
    }
  }
  return -1;
}

/**
 * 处理BMS 矩阵的 Ω下标
 * @param {Matrix} M
 * @param {number} n
 * @returns {Term}
 */
function getAdmIndexOfMatrix(M, n) {
  // 对于xxxxx(1,0)，为ω系序数，返回0
  if (M[n][1] == 0) {
    return [];
  }
  if (M[n][2] == 0) {
    // 可能是前有Ω_ω列，需要检查,比如说[0,0,0][1,1,1][2,1][3,2]
    // 通常检查BO以下BMS u默认为1

    // 对于(0,0,0)(1,1,1)(2,1)(1,1,1), 此处的1会被覆盖为查找omegalike后的index，进行matrixxth后 取最后一项

    let upgradingTermAdm =
      checkW_wLikeFunction(M, n) >= 0
        ? lastTerm(getAdmIndexOfMatrix(M, checkW_wLikeFunction(M, n)))
        : ONE;
    return add(
      getAdmIndexOfMatrix(M, findMatrixParentTerm(M, 1, n)),
      upgradingTermAdm,
    );
  }
  let omega_power_x_counter = ONE;
  //数(0,0,0)(1,1,1)(2,1,1)的
  for (i of children(M, n)) {
    if (!eq(M[i], [M[n][0] + 1, M[n][1], 1])) {
      continue;
    }
    let q = [];
    for (j of children(M, i)) {
      q = add(q, matrixToBMS2(M, j));
    }
    omega_power_x_counter = add(omega_power_x_counter, exp(q));
  }
  return add(
    getAdmIndexOfMatrix(M, findMatrixParentTerm(M, 1, n)),
    exp(omega_power_x_counter),
  );
}

/**
 * 主要的矩阵转换成序数的函数
 * @param {Matrix} M
 * @param {number} n
 * @returns {Term}
 */
function matrixToBMS2(M, n) {
  /**
   * @type {Term}
   */
  let omegaMultiplication = [];
  let u = [...Array(M.length).keys()].map((x) => checkW_wLikeFunction(M, x));
  /**如果没有找到此列是坏根的列，就说明遇到(0)(1) index=1或者(0)(1)(2)(1), index=2这个情况 */

  for (i of children(M, n)) {
    // 如果满足 x y z的子序列有x+1 y 1， 或者 对于(2,1,0)(1,1,1)特殊情况 那么不计入
    // 忽略处理会发现 Ω_{ω^2}*Ω_{ω}, Ω_{ω}^2+Ω_{ω}等不合理情况
    // 例如 1 1 1, 2 1 1
    if (eq(M[i], [M[n][0] + 1, M[n][1], 1])) {
      continue;
    }
    if (u.includes(i)) {
      let c = children(M, i);
      if (c.length) {
        if (eq(M[c.at(-1)], [M[i][0] + 1, M[i][1], 1])) {
          continue;
        }
      } else {
        continue;
      }
    }
    //对psiInner内的序数进行递归处理，
    omegaMultiplication = add(omegaMultiplication, matrixToBMS2(M, i));
  }
  return [getAdmIndexOfMatrix(M, n), omegaMultiplication, []];
}

/**
 * 把一个矩阵转换成Term形式
 * @param {Matrix} M
 * @returns {Term}
 */
function matrixToBMS(M) {
  let S = [];
  for (let i = 0; i < M.length; i++) {
    /**把整个矩阵分成(0,0,0)xxxx(0,0,0)xxx */
    if (eq(M[i], [0, 0, 0])) {
      S = add(S, matrixToBMS2(M, i));
    }
  }
  return standardForm(S);
}

//计算矩阵对应的(未标准化的)Extended B-Hydra表达式
function NS(M) {
  let S = [];
  for (let i = 0; i < M.length; i++) {
    if (eq(M[i], [0, 0, 0])) {
      S = add(S, matrixToBMS2(M, i));
    }
  }
  return S;
}

/*部分标准化
给定a,b,c,满足ψa(b)和c均为标准式,该函数将ψa(b+c)标准化
注:我们称ψa(b)为标准的,当且仅当a,b均为标准的,且对于任意c>b,有ψa(c)>ψa(b)
该程序以递归的方式进行,从左到右对c的每一项进行标准化,并将其添加到b中
这也启发我们"一项一项地计算"
因此,我们考虑一个问题:ψa(ψb(c))什么时候是标准的?如果不标准,在ψb(c)的前面添加一个什么样的序数就标准了?
首先,当b<ψa(ψa+1(0))时,ψa(b)=Ω_a*ω^b一定是标准的
*/
function sp(a, b, c) {
  if (isZero(c)) {
    return [a, b, []];
  }
  //设c=ψd(t+h)+f,t的所有项大于等于ψd+1(0)
  if (lt(b, c[1]) && gt(c, [a, [], []])) {
    //b<t+h且d>=b
    let t = truncate(c[1], succ(c[0]));
    return sp(a, add(t, sub(firstTerm(c), [c[0], t, []])), c[2]);
    //ψa(b+ψd(t+h))标准化为ψa(t+{ψd(t+h)-ψd(t)})
    //若c=ψd(t),结果为ψa(t),相当于删去中间层
    //若c>ψd(t),结果为ψa(t+ψd(t+h))
  }
  return sp(a, add(b, firstTerm(c)), c[2]);
}

//真正的标准化函数
function standardForm(a) {
  if (isZero(a)) {
    return [];
  }
  return add(
    sp(standardForm(a[0]), [], standardForm(a[1])),
    standardForm(a[2]),
  );
}

function createTable(X) {
  return X.map(
    (x) => "<tr>" + x.map((y) => "<td>" + y + "</td>").join("") + "</tr>",
  ).join("");
}

//将ψa(x)(a>0)转化为Ω_a^b*c的形式
function g(a) {
  if (isZero(a)) {
    return [[], []];
  }
  if (isZero(a[0])) {
    return [log(a), []];
  }
  let [p, s] = separate(a[1], [succ(a[0]), [], []]);
  let [q, r] = separate(s, [a[0], [], []]);
  //令x=p+q+r,其中p每一项大于ψb+1(0),q每一项大于ψb(0)
  let second = exp(r);
  let first = add(ONE, p);
  let ptr = q;
  while (!isZero(ptr)) {
    ((first = add(first, exp(sub(log(ptr), [a[0], [], []])))), (ptr = ptr[2]));
  }
  return [first, second];
}

//Ω_a的简写
function omega(a) {
  if (isZero(a)) return "\\omega";
  if (eq(a, ONE)) return "\\Omega";
  return `\\Omega_{${termToString2(a)}}`;
}

/**
 * 化为可读的字符串
 * @param {Term} q
 * @returns {string}
 */
function termToString2(q) {
  if (isZero(q)) {
    return "0";
  }
  if (isOrdinalFinite(q)) {
    return length1(q).toString();
  }
  let [a, b] = separate(q, firstTerm(q));
  let m = `\\psi_{${termToString2(a[0])}}(${termToString2(a[1])})`;
  if (isZero(a[1])) {
    m = omega(a[0]);
  }
  // if(isZero(a[1])){m=`Ω<sub>${termToString(a[0])}</sub>`;}  //a>0时，ψa(0)=Ω_a
  // if(isZero(a[1])&&eq(a[0],ONE)){m=`Ω`;}  //Ω_1简写为Ω
  if (isZero(a[0])) {
    m = `\\psi(${termToString2(a[1])})`;
  } //ψ0(x)简写为ψ(x)
  if (eq(a[0], []) && eq(a[1], ONE)) {
    m = "\\omega";
  } //ψ(1)=ω
  else if (lt(a[1], [succ(a[0]), [], []])) {
    let [first, second] = g(a);
    m = omega(a[0]);
    if (gt(first, ONE)) {
      m += `^{${termToString2(first)}}`;
    }
    if (gt(second, ONE)) m += termToString2(second);
  }
  //else if(!eq(log(firstTerm(a)),firstTerm(a))){m=`ω<sup>${termToString(log(a))}</sup>`;}
  //  else if(!le(lastTerm(a[1]),[succ(a[0]),[],[]])&&le(lastTerm(a[1]),[succ(a[0]),[succ(a[0]),[],[]],[]])){
  //    let [f,g]=separate(a[1],[succ(a[0]),[succ(a[0]),[],[]],[]]);
  //  }
  //a的每一项均为q的主项，故一定形如ψb(c)*n，这里计算系数n
  if (length1(a) > 1) {
    m += length1(a);
  }
  if (!isZero(b)) {
    m += `+${termToString2(b)}`;
  }
  return m;
} /**
 * 化为可读的字符串
 * @param {Term} q
 * @returns {string}
 */
function termToString(q) {
  return katex.renderToString(termToString2(q), {
    throwOnError: false,
  });
}

const EBO = [
  [0, 0, 0],
  [1, 1, 1],
  [2, 1, 1],
  [3, 1, 0],
  [2, 0, 0],
];

/**
 *
 * @param {number[][]} a
 * @param {number[][]} b
 * @returns {0|1|-1}
 */
function zidianxu(a, b) {
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    for (let j = 0; j < Math.max(a.length, b.length); j++) {
      if ((a[i]?.[j] ?? 0) > (b[i]?.[j] ?? 0)) return 1;
      if ((a[i]?.[j] ?? 0) < (b[i]?.[j] ?? 0)) {
        return -1;
      }
    }
  }
  return 0;
}
function calculate() {
  let M = document.getElementById("input").value;
  M = M.replace(/[^\(\)（），,\d]/g, "");
  M = M.replace(/（/g, "(");
  M = M.replace(/）/g, ")");
  M = M.replace(/，/g, ",");
  try {
    M = eval(
      "[" +
        M.replaceAll(")(", "],[").replaceAll("(", "[").replaceAll(")", "]") +
        "]",
    );
  } catch (e) {
    return;
  }
  M = M.map((x) => {
    let y = x.slice();
    while (y.length < 3) {
      y.push(0);
    }
    return y;
  });
  let A = [...Array(M.length).keys()].map((x) => D(M, x));
  if (Math.max(...A) > 15) {
    document.getElementById("output").innerHTML = "Too complex";
    document.getElementById("output3").innerHTML = "";
    let Q =
      '<tr><th class="border">i</th><th class="border" colspan=3>M<sub>i</sub></th><th class="border">matrixToBMS2(M,i)</th><th class="border">getAdmIndexOfMatrix(M,i)</th><th class="border">checkW_wLikeFunction(M,i)</th><th class="border">Children</th>';
    for (let i = 0; i < M.length; i++) {
      Q += "<tr>";
      let m = [
        i.toString(),
        "(" + M[i][0] + ",",
        M[i][1] + ",",
        M[i][2] + ")",
        "?",
        "?",
        "?",
        "?",
      ];
      for (let j = 0; j < m.length; j++) {
        if (j == 1 || j == 2 || j == 3) {
          Q += '<td class="nborder">';
        } else {
          Q += '<td class="border">';
        }
        Q += `${m[j]}</td>`;
      }
      Q += "</tr>";
    }
    Q += `<tr><td>Σ</td><td colspan=7>?</td></tr>`;
    document.getElementById("output2").innerHTML = Q;
    return;
  }
  const gteEBO =
    zidianxu(M, EBO) >= 0 ||
    (function () {
      for (let i = 0; i < M.length; i++) {
        if (M[i][2] >= 2) return true;
        for (let j = 3; j < M[i].length; j++) {
          if (M[i][j] >= 1) return true;
        }
      }
      return false;
    })();

  if (gteEBO) {
    document.getElementById("output").innerHTML = "≥EBO";
  } else {
    document.getElementById("output").innerHTML = termToString(matrixToBMS(M));
    document.getElementById("output3").innerHTML = eq(NS(M), matrixToBMS(M))
      ? ""
      : "<i>n.s.</i> " + termToString(NS(M));
  }
  let Q =
    '<tr><th class="border">i</th><th class="border" colspan=3>M<sub>i</sub></th><th class="border">matrixToBMS2(M,i)</th><th class="border">getAdmIndexOfMatrix(M,i)</th><th class="border">checkW_wLikeFunction(M,i)</th><th class="border">Children</th>';
  let u = [...Array(M.length).keys()].map((x) => checkW_wLikeFunction(M, x));
  for (let i = 0; i < M.length; i++) {
    Q += "\n";
    if (eq(M[i], [0, 0, 0])) {
      Q += '<tr style="background-color:cyan">';
    } else if (u.includes(i)) {
      let c = children(M, i);
      if (c.length) {
        if (eq(M[c.at(-1)], [M[i][0] + 1, M[i][1], 1])) {
          Q += '<tr style="color:#bbb;background-color:yellow">';
        } else {
          Q += '<tr style="background-color:lime">';
        }
      } else {
        Q += '<tr style="color:#bbb;background-color:yellow">';
      }
    } else if (
      M[i][2] == 1 &&
      eq(M[findMatrixParentTerm(M, 0, i)], [M[i][0] - 1, M[i][1], 1])
    ) {
      Q += '<tr style="color:#bbb;">';
    } else {
      Q += "<tr>";
    }
    let m = [
      i.toString(),
      "(" + M[i][0] + ",",
      M[i][1] + ",",
      M[i][2] + ")",
      gteEBO ? "?" : termToString(matrixToBMS2(M, i)),
      gteEBO ? "?" : termToString(getAdmIndexOfMatrix(M, i)),
      checkW_wLikeFunction(M, i) != -1
        ? checkW_wLikeFunction(M, i).toString()
        : "-",
      children(M, i),
    ];
    for (let j = 0; j < m.length; j++) {
      if (j == 1 || j == 2 || j == 3) {
        Q += '<td class="nborder">';
      } else {
        Q += '<td class="border">';
      }
      Q += `${m[j]}</td>`;
    }
    Q += "</tr>";
  }
  Q += `<tr><td>Σ</td><td colspan=7>${
    gteEBO ? "?" : termToString(NS(M))
  }</td></tr>`;
  document.getElementById("output2").innerHTML = Q;
}
document.getElementById("input").value = "(0,0,0)(1,1,1)(2,1,0)(1,1,1)";
calculate();
