
use bms_core::bms::{bms_to_bocf, is_eq_ebo, is_gte_ebo};
use bms_core::expand::expand_bms;
use bms_core::hydra::{
    expand_hprss, expand_hydra, expand_lprss, format_hydra_psi, hydra_to_bms, hydra_to_bocf,
    hydra_to_hprss, hydra_to_hprss_standard, hydra_to_lprss, hprss_to_hydra, lprss_to_hydra,
    normalize_hydra, parse_hydra, term_to_hydra,
};
use bms_core::parser::{bocf_to_bms, eval_ast, parse_bocf};
use bms_core::term::*;
use bms_core::triangular::{bms_to_triangular, triangular_to_bms};
use bms_core::wy::{expand_1y, expand_wy_seq};
use bms_core::zero_y::{bms_to_0y_sequence, zero_y_expand, zero_y_to_bms};
use bms_core::Matrix;
use std::env;
use std::process::exit;

// ── Matrix helpers ──

fn parse_matrix(s: &str) -> Matrix {
    let mut m: Matrix = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'(' {
            break;
        }
        let rest = &s[i..];
        match rest.find(')') {
            None => break,
            Some(end) => {
                let col_str = &rest[1..end];
                let mut row = Vec::new();
                for part in col_str.split(',') {
                    let part = part.trim();
                    if let Ok(v) = part.parse::<i32>() {
                        row.push(v);
                    }
                }
                m.push(row);
                i += end + 1;
            }
        }
    }
    m
}

fn format_matrix(m: &Matrix) -> String {
    let mut s = String::new();
    for col in m {
        s += "(";
        for (i, v) in col.iter().enumerate() {
            if i > 0 {
                s += ",";
            }
            s += &v.to_string();
        }
        s += ")";
    }
    s
}

fn parse_seq(s: &str) -> Vec<i32> {
    let mut seq = Vec::new();
    for part in s.split(',') {
        let p = part.trim();
        match p.parse::<i32>() {
            Ok(v) => seq.push(v),
            Err(_) => {
                eprintln!("Error: invalid number '{}' in sequence", p);
                exit(1);
            }
        }
    }
    seq
}

fn format_seq(seq: &[i32]) -> String {
    seq.iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

// ── LaTeX stripping ──

fn strip_latex(s: &str) -> String {
    let mut r = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                j += 1;
            }
            let cmd = &s[i + 1..j];
            match cmd {
                "psi" => r += "ψ",
                "Omega" => r += "Ω",
                "omega" => r += "ω",
                "varepsilon" => r += "ε",
                "zeta" => r += "ζ",
                "eta" => r += "η",
                "Gamma" => r += "Γ",
                "varphi" => r += "φ",
                "left" | "right" => {}
                _ => r += cmd,
            }
            while j < bytes.len() && bytes[j] == b'{' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'}' {
                j += 1;
            }
            i = j;
        } else {
            r.push(bytes[i] as char);
            i += 1;
        }
    }
    r
}

// ── Output ──

enum OutputType {
    All,
    Bocf,
    Bms,
    Veblen,
    ZeroY,
    Triangular,
    Hydra,
    Hprss,
    Lprss,
}

fn parse_output_type(s: &str) -> OutputType {
    match s {
        "bocf" | "ocf" | "ordinal" => OutputType::Bocf,
        "bms" => OutputType::Bms,
        "veblen" | "φ" | "phi" => OutputType::Veblen,
        "0y" | "0-y" | "seq" => OutputType::ZeroY,
        "triangular" => OutputType::Triangular,
        "hydra" | "pss" => OutputType::Hydra,
        "hprss" => OutputType::Hprss,
        "lprss" => OutputType::Lprss,
        _ => OutputType::All,
    }
}

fn print_bocf(ordinal: &Term, latex_mode: bool) {
    let s = term_to_string(false, ordinal);
    let s = if latex_mode { s } else { strip_latex(&s) };
    println!("BOCF: {}", s);
}

fn print_veblen(ordinal: &Term, latex_mode: bool) {
    let v = if latex_mode {
        term_to_veblen(ordinal)
    } else {
        term_to_veblen_plain(ordinal)
    };
    if !v.is_empty() {
        let v = if latex_mode { v } else { strip_latex(&v) };
        println!("Veblen: {}", v);
    }
}

fn print_0y(m: &Matrix) {
    let s = bms_to_0y_sequence(m);
    if !s.is_empty() {
        println!("0-Y: {}", s);
    }
}

fn print_bms(m: &Matrix) {
    println!("BMS: {}", format_matrix(m));
}

fn print_tri_bms(m: &Matrix) {
    let tri = bms_to_triangular(m);
    if !tri.is_empty() {
        println!("Triangular BMS: {}", format_matrix(&tri));
    }
}

// ── Commands ──

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn flag_value(args: &[String], flag: &str) -> Option<i32> {
    for (i, a) in args.iter().enumerate() {
        if a == flag && i + 1 < args.len() {
            return args[i + 1].parse().ok();
        }
    }
    None
}

fn cmd_bms(args: &[String]) -> i32 {
    let latex_mode = has_flag(args, "--latex");
    let input = &args[0];
    let mut m = parse_matrix(input);
    if m.is_empty() {
        eprintln!("Error: could not parse matrix");
        return 1;
    }

    // Auto-detect triangular BMS
    let is_zero = |c: &Vec<i32>, up_to: usize| -> bool {
        for i in 1..=up_to {
            if i < c.len() && c[i] != 0 {
                return false;
            }
        }
        true
    };
    let mut is_triangular_input = m.len() >= 3
        && m[0].len() >= 1
        && m[0][0] == 0
        && is_zero(&m[0], 2)
        && m[1].len() >= 1
        && m[1][0] == 1
        && is_zero(&m[1], 2)
        && m[2].len() >= 2
        && m[2][0] == 2
        && m[2][1] == 1
        && (m[2].len() <= 2 || m[2][2] == 0);

    if has_flag(args, "--triangular") {
        is_triangular_input = true;
    }

    let mut tri_form: Matrix = Vec::new();
    if is_triangular_input {
        tri_form = m.clone();
        m = triangular_to_bms(&m);
    }

    if let Some(fs) = flag_value(args, "--expand") {
        let result = expand_bms(&m, fs);
        println!("{}", format_matrix(&result));
        return 0;
    }

    let mut out = OutputType::All;
    for (i, a) in args.iter().enumerate() {
        if a == "--to" && i + 1 < args.len() {
            out = parse_output_type(&args[i + 1]);
            break;
        }
    }

    if matches!(out, OutputType::Triangular) {
        let t = if is_triangular_input {
            tri_form.clone()
        } else {
            bms_to_triangular(&m)
        };
        println!("{}", format_matrix(&t));
        return 0;
    }

    if is_gte_ebo(&m) {
        let label = if is_eq_ebo(&m) { "\\psi(I)" } else { ">\\psi(I)" };
        match out {
            OutputType::All => {
                if !tri_form.is_empty() {
                    print_bms(&m);
                }
                println!("BOCF: {}", label);
                print_0y(&m);
                if tri_form.is_empty() {
                    tri_form = bms_to_triangular(&m);
                    println!("Triangular BMS: {}", format_matrix(&tri_form));
                }
            }
            OutputType::Bocf => {
                println!("{}", label);
            }
            OutputType::ZeroY => print_0y(&m),
            OutputType::Lprss => eprintln!("Error: ordinal is beyond the LPrSS limit φ(ω,0)"),
            OutputType::Bms => print_bms(&m),
            _ => {}
        }
        return 0;
    }

    let ordinal = bms_to_bocf(&m);

    match out {
        OutputType::All => {
            if !tri_form.is_empty() {
                print_bms(&m);
            }
            print_bocf(&ordinal, latex_mode);
            print_veblen(&ordinal, latex_mode);
            print_0y(&m);
            if tri_form.is_empty() {
                tri_form = bms_to_triangular(&m);
                println!("Triangular BMS: {}", format_matrix(&tri_form));
            }
        }
        OutputType::Bocf => print_bocf(&ordinal, latex_mode),
        OutputType::Bms => print_bms(&m),
        OutputType::Veblen => print_veblen(&ordinal, latex_mode),
        OutputType::ZeroY => print_0y(&m),
        OutputType::Lprss => {
            match term_to_hydra(&ordinal).and_then(|h| hydra_to_lprss(&h)) {
                Ok(seq) => println!("LPrSS: {}", format_seq(&seq)),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
        OutputType::Triangular => {}
        _ => {}
    }
    0
}

fn cmd_bocf(args: &[String]) -> i32 {
    let latex_mode = has_flag(args, "--latex");
    let input = &args[0];

    // --expand flag first
    if let Some(n) = flag_value(args, "--expand") {
        let ast = match parse_bocf(input) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Parse error: {}", e);
                return 1;
            }
        };
        let val = match eval_ast(&ast) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Eval error: {}", e);
                return 1;
            }
        };
        let val = standard_form(&val);
        let fs = fundamental_sequence(&val, n);
        let s = term_to_string(false, &fs);
        let s = if latex_mode { s } else { strip_latex(&s) };
        println!("{}", s);
        return 0;
    }

    // Normal BOCF→BMS path
    eprintln!("Converting BOCF to BMS...");
    let result = bocf_to_bms(input, &mut |s: &str| {
        eprint!("\r  iter {}", s);
    });
    eprintln!();
    let result = match result {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };

    let m = parse_matrix(&result);
    if m.is_empty() {
        println!("{}", result);
        return 0;
    }

    let mut out = OutputType::All;
    for (i, a) in args.iter().enumerate() {
        if a == "--to" && i + 1 < args.len() {
            out = parse_output_type(&args[i + 1]);
            break;
        }
    }

    if matches!(out, OutputType::Bms) {
        println!("{}", result);
        return 0;
    }

    let ordinal = bms_to_bocf(&m);

    match out {
        OutputType::All => {
            print_bms(&m);
            print_tri_bms(&m);
            print_veblen(&ordinal, latex_mode);
            print_0y(&m);
        }
        OutputType::Bocf => print_bocf(&ordinal, latex_mode),
        OutputType::Veblen => print_veblen(&ordinal, latex_mode),
        OutputType::ZeroY => print_0y(&m),
        _ => println!("{}", result),
    }
    0
}

fn cmd_0y(args: &[String]) -> i32 {
    let latex_mode = has_flag(args, "--latex");
    let input = &args[0];

    if let Some(n) = flag_value(args, "--expand") {
        let seq = parse_seq(input);
        let expanded = zero_y_expand(&seq, n);
        println!("{}", format_seq(&expanded));
        return 0;
    }

    let seq = parse_seq(input);
    let m = zero_y_to_bms(&seq);

    let mut out = OutputType::All;
    for (i, a) in args.iter().enumerate() {
        if a == "--to" && i + 1 < args.len() {
            out = parse_output_type(&args[i + 1]);
            break;
        }
    }

    match out {
        OutputType::All => {
            print_bms(&m);
            print_tri_bms(&m);
            if is_gte_ebo(&m) {
                let label = if is_eq_ebo(&m) { "\\psi(I)" } else { ">\\psi(I)" };
                println!("BOCF: {}", label);
            } else {
                let ordinal = bms_to_bocf(&m);
                print_bocf(&ordinal, latex_mode);
                print_veblen(&ordinal, latex_mode);
            }
        }
        OutputType::Bms => print_bms(&m),
        OutputType::Bocf => {
            if is_gte_ebo(&m) {
                let label = if is_eq_ebo(&m) { "\\psi(I)" } else { ">\\psi(I)" };
                println!("{}", label);
            } else {
                let ordinal = bms_to_bocf(&m);
                print_bocf(&ordinal, latex_mode);
            }
        }
        OutputType::Veblen => {
            if !is_gte_ebo(&m) {
                let ordinal = bms_to_bocf(&m);
                print_veblen(&ordinal, latex_mode);
            }
        }
        _ => {}
    }
    0
}

fn cmd_1y(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Usage: analyzer-cli 1y <sequence> [--expand <n>]");
        return 1;
    }
    let input = &args[0];
    let fs = match flag_value(args, "--expand") {
        Some(f) => f,
        None => {
            eprintln!("Error: --expand is required");
            return 1;
        }
    };
    let seq = parse_seq(input);
    let expanded = expand_1y(&seq, fs);
    println!("{}", format_seq(&expanded));
    0
}

fn cmd_wy(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Usage: analyzer-cli wy <sequence> [--expand <n>]");
        return 1;
    }
    let input = &args[0];
    let fs = match flag_value(args, "--expand") {
        Some(f) => f,
        None => {
            eprintln!("Error: --expand is required");
            return 1;
        }
    };
    let seq = parse_seq(input);
    let expanded = expand_wy_seq(&seq, fs);
    println!("{}", format_seq(&expanded));
    0
}

fn cmd_hprss(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Usage: analyzer-cli hprss <sequence> [--expand <n>] [--to <fmt>]");
        return 1;
    }
    let input = &args[0];
    let seq = parse_seq(input);
    if seq.is_empty() || seq[0] < 1 {
        eprintln!("Error: HPrSS sequence must start with a positive integer");
        return 1;
    }

    if let Some(n) = flag_value(args, "--expand") {
        let expanded = expand_hprss(&seq, n);
        println!("{}", format_seq(&expanded));
        return 0;
    }

    let mut out = OutputType::All;
    for (i, a) in args.iter().enumerate() {
        if a == "--to" && i + 1 < args.len() {
            out = parse_output_type(&args[i + 1]);
            break;
        }
    }

    let h = normalize_hydra(&hprss_to_hydra(&seq));
    match out {
        OutputType::All => {
            println!("PSS Hydra: {}", format_hydra_psi(&h));
            print_hydra_ordinal(&hprss_to_hydra(&seq));
            let bms = hydra_to_bms(&h);
            match bms {
                Ok(m) => {
                    println!("BMS: {}", format_matrix(&m));
                    let zero_y = bms_to_0y_sequence(&m);
                    if !zero_y.is_empty() {
                        println!("0-Y: {}", zero_y);
                    }
                }
                Err(e) => println!("BMS: (error: {})", e),
            }
        }
        OutputType::Hydra => println!("{}", format_hydra_psi(&h)),
        OutputType::Hprss => println!("{}", format_seq(&seq)),
        OutputType::Bms => match hydra_to_bms(&h) {
            Ok(m) => println!("{}", format_matrix(&m)),
            Err(e) => println!("(error: {})", e),
        },
        OutputType::Bocf => {
            let ordinal = hydra_to_bocf(&hprss_to_hydra(&seq));
            print_bocf(&ordinal, false);
        }
        OutputType::Veblen => {
            let ordinal = hydra_to_bocf(&hprss_to_hydra(&seq));
            print_veblen(&ordinal, false);
        }
        _ => {}
    }
    0
}

fn cmd_lprss(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Usage: analyzer-cli lprss <sequence> [--expand <n>] [--to <fmt>]");
        return 1;
    }
    let seq = parse_seq(&args[0]);
    if seq.is_empty() || seq[0] < 1 {
        eprintln!("Error: LPrSS sequence must start with a positive integer");
        return 1;
    }
    if let Some(n) = flag_value(args, "--expand") {
        println!("{}", format_seq(&expand_lprss(&seq, n)));
        return 0;
    }
    let h = lprss_to_hydra(&seq);
    let mut out = OutputType::Bocf;
    for (i, a) in args.iter().enumerate() {
        if a == "--to" && i + 1 < args.len() {
            out = parse_output_type(&args[i + 1]);
            break;
        }
    }
    match out {
        OutputType::Hydra => println!("{}", format_hydra_psi(&normalize_hydra(&h))),
        OutputType::Bms => print_bms(&hydra_to_bms(&h).unwrap()),
        OutputType::Veblen => print_veblen(&hydra_to_bocf(&h), false),
        _ => print_bocf(&hydra_to_bocf(&h), false),
    }
    0
}

fn cmd_hydra(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Usage: analyzer-cli hydra <expr> [--expand <n>] [--to <fmt>]");
        return 1;
    }
    let input = &args[0];
    let h = match parse_hydra(input) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return 1;
        }
    };

    if let Some(n) = flag_value(args, "--expand") {
        let norm = normalize_hydra(&h);
        match expand_hydra(&norm, n) {
            Ok(e) => println!("{}", format_hydra_psi(&e)),
            Err(e) => {
                eprintln!("Error: {}", e);
                return 1;
            }
        }
        return 0;
    }

    let mut out = OutputType::All;
    for (i, a) in args.iter().enumerate() {
        if a == "--to" && i + 1 < args.len() {
            out = parse_output_type(&args[i + 1]);
            break;
        }
    }

    let norm = normalize_hydra(&h);
    match out {
        OutputType::All => {
            println!("PSS Hydra: {}", format_hydra_psi(&norm));
            print_hydra_ordinal(&h);
            let hprss = hydra_to_hprss_standard(&h);
            if !hprss.is_empty() {
                println!("HPrSS: {}", format_seq(&hprss));
            }
            let bms = hydra_to_bms(&norm);
            match bms {
                Ok(m) => {
                    println!("BMS: {}", format_matrix(&m));
                    let zero_y = bms_to_0y_sequence(&m);
                    if !zero_y.is_empty() {
                        println!("0-Y: {}", zero_y);
                    }
                }
                Err(e) => println!("BMS: (error: {})", e),
            }
        }
        OutputType::Hydra => println!("{}", format_hydra_psi(&norm)),
        OutputType::Hprss => {
            let hprss = hydra_to_hprss_standard(&h);
            println!("{}", format_seq(&hprss));
        }
        OutputType::Bms => match hydra_to_bms(&norm) {
            Ok(m) => println!("{}", format_matrix(&m)),
            Err(e) => println!("(error: {})", e),
        },
        OutputType::Bocf => {
            let ordinal = hydra_to_bocf(&h);
            print_bocf(&ordinal, false);
        }
        OutputType::Veblen => {
            let ordinal = hydra_to_bocf(&h);
            print_veblen(&ordinal, false);
        }
        _ => {}
    }
    0
}

fn print_hydra_ordinal(h: &bms_core::hydra::Hydra) {
    let ordinal = hydra_to_bocf(h);
    print_bocf(&ordinal, false);
    print_veblen(&ordinal, false);
}

fn cmd_fs(args: &[String]) -> i32 {
    let latex_mode = has_flag(args, "--latex");
    if args.len() < 2 {
        eprintln!("Usage: analyzer-cli fs <expr> <n>");
        return 1;
    }
    let input = &args[0];
    let n: i32 = match args[1].parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Usage: analyzer-cli fs <expr> <n>");
            return 1;
        }
    };
    let ast = match parse_bocf(input) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return 1;
        }
    };
    let val = match eval_ast(&ast) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Eval error: {}", e);
            return 1;
        }
    };
    let val = standard_form(&val);
    let fs = fundamental_sequence(&val, n);
    let s = term_to_string(false, &fs);
    let s = if latex_mode { s } else { strip_latex(&s) };
    println!("{}", s);
    0
}

fn print_usage() {
    println!(
        "Usage:\n\
  analyzer-cli bms <matrix> [--to <fmt>] [--expand <fs>] [--triangular]\n\
  analyzer-cli bocf <expr> [--to <fmt>] [--expand <n>]\n\
  analyzer-cli 0y <sequence> [--to <fmt>] [--expand <n>]\n\
  analyzer-cli 1y <sequence> --expand <n>\n\
  analyzer-cli wy <sequence> --expand <n>\n\
  analyzer-cli hprss <sequence> [--to <fmt>] [--expand <n>]\n\
  analyzer-cli hydra <expr> [--to <fmt>] [--expand <n>]\n\
  analyzer-cli fs <expr> <n>   Fundamental sequence of a BOCF expression\n\
\n\
Commands:\n\
  bms   Analyze or expand a BMS matrix (or triangular BMS with --triangular)\n\
  bocf  Convert BOCF expression to BMS (or expand with --expand)\n\
  0y    Convert 0-Y sequence to BMS (or expand with --expand)\n\
  1y    Expand a 1-Y sequence\n\
  wy    Expand a ω-Y sequence\n\
  hprss Expand or convert an HPrSS sequence\n\
  hydra Expand or convert a PSS Hydra expression (p1(p2(0)))\n\
  fs    Compute the nth term of an ordinal's fundamental sequence\n\
\n\
Flags:\n\
  --to <fmt>     Output a specific notation (bocf, bms, veblen, 0y, triangular, hydra, hprss)\n\
  --expand <n>   Expand BMS matrix or compute FS of BOCF (n = step)\n\
  --triangular   Treat input matrix as triangular BMS (convert to standard before analysis)\n\
  --latex        Output LaTeX formatting for Veblen\n\
\n\
Formats:\n\
  Matrix:  (0,0,0)(1,1,1)(2,2,0)\n\
  0-Y:     1,2,3,4\n\
  HPrSS:   1,4,6,6\n\
  PSS Hydra: p1(p2(0)+p2(0))\n\
  BOCF:    p(w)  (p=ψ, w=ω, W=Ω)"
    );
}

fn main() {
    let argv: Vec<String> = env::args().collect();
    if argv.len() < 2 {
        print_usage();
        exit(1);
    }

    let cmd = &argv[1];

    if cmd == "-h" || cmd == "--help" {
        print_usage();
        exit(0);
    }

    let cmd_args: Vec<String> = argv[2..].to_vec();
    if cmd_args.is_empty() {
        print_usage();
        exit(1);
    }

    let code = match cmd.as_str() {
        "bms" => cmd_bms(&cmd_args),
        "bocf" => cmd_bocf(&cmd_args),
        "0y" => cmd_0y(&cmd_args),
        "1y" => cmd_1y(&cmd_args),
        "wy" => cmd_wy(&cmd_args),
        "hprss" => cmd_hprss(&cmd_args),
        "lprss" => cmd_lprss(&cmd_args),
        "hydra" => cmd_hydra(&cmd_args),
        "fs" => cmd_fs(&cmd_args),
        _ => {
            eprintln!("Unknown command: {}\n", cmd);
            print_usage();
            1
        }
    };
    exit(code);
}
