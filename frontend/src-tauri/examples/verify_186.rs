use app_lib::summary::hard_post_process as hpp;

fn safe_print(label: &str, s: &str, n: usize) {
    let limit = n.min(s.len());
    // floor to char boundary
    let mut safe = limit;
    while safe > 0 && !s.is_char_boundary(safe) {
        safe -= 1;
    }
    println!("\n=== {} ({} bytes) ===", label, safe);
    println!("{}", &s[..safe]);
}

fn main() {
    let transcript = std::fs::read_to_string(
        "/Users/wangwei/Downloads/高压触电致人损害责任纠纷案件审理报告_2026-08-19.txt"
    ).unwrap();
    let summary = std::fs::read_to_string(
        "/Users/wangwei/Downloads/高压触电致人损害责任纠纷案庭审报告_2026-08-19_summary.md"
    ).unwrap();

    println!("=== transcript chars: {} ===", transcript.len());
    println!("=== summary chars: {} ===", summary.len());

    let extracted = hpp::extract_party_roles_from_transcript(&transcript);
    println!("\n=== §185.1 EXTRACT (REAL DATA) ===");
    println!("deceased: {:?}", extracted.deceased);
    println!("plaintiffs: {:?}", extracted.plaintiffs);
    println!("defendants: {:?}", extracted.defendants);
    println!("role_warnings: {:?}", extracted.role_warnings);

    let (fixed_md, fix_report) = hpp::fix_party_role_conflict_in_markdown(&summary, &extracted);
    println!("\n=== §186.1 FIX (REAL DATA) ===");
    println!("fixed lines count: {}", fix_report.fixed_lines.len());
    for line in fix_report.fixed_lines.iter().take(10) {
        println!("  - {}", line);
    }
    // §186.1 FIX: 显示改名后的关键行 (含原告/被告)
    println!("\n=== 改名后相关行 ===");
    for line in fixed_md.lines() {
        if line.contains("温明仁") || line.contains("⚠️") {
            println!("[RENAMED] <{}>", line);
        }
    }
    safe_print("first 2000 chars of fixed_md", &fixed_md, 2000);

    let (asr_fixed, asr_fixes) = hpp::fix_asr_transcription_errors(&fixed_md);
    println!("\n=== §186.2 ASR (REAL DATA) ===");
    println!("fixes count: {}", asr_fixes.len());
    for f in asr_fixes.iter().take(15) {
        println!("  - {}", f);
    }
    println!("\n剩余 '双方军和' 数: {}",
        asr_fixed.matches("双方军和").count()
    );

    let statute_report = hpp::check_statute_completeness(&asr_fixed, &transcript);
    println!("\n=== §186.3 STATUTE (REAL DATA) ===");
    println!("is_high_voltage_case: {}", statute_report.is_high_voltage_case);
    println!("missing: {:?}", statute_report.missing_required_statutes);
}
