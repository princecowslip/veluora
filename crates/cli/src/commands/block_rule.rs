use application::BlockRuleService;
use domain::{BlockRuleId, RuleType, Scope};
use uuid::Uuid;

use super::{open_context, print_error_message, print_json, report_and_exit};
use crate::cli_args::OutputFormat;
use crate::exit_code::ExitCode;

fn parse_block_rule_id(
    format: OutputFormat,
    quiet: bool,
    raw: &str,
) -> Result<BlockRuleId, ExitCode> {
    Uuid::parse_str(raw).map(BlockRuleId).map_err(|_| {
        print_error_message(format, quiet, &format!("invalid block rule id: '{raw}'"));
        ExitCode::InvalidArguments
    })
}

pub fn list(format: OutputFormat, quiet: bool) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match BlockRuleService::list(&ctx) {
        Ok(rules) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&rules),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        if rules.is_empty() {
                            println!("(no block rules configured)");
                        }
                        for rule in &rules {
                            println!(
                                "{}  {:?}  target={}  scope={:?}  enabled={}",
                                rule.id, rule.rule_type, rule.target, rule.scope, rule.enabled
                            );
                        }
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn add(
    format: OutputFormat,
    quiet: bool,
    rule_type: RuleType,
    target: String,
    scope: Scope,
    reason: Option<String>,
) -> ExitCode {
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match BlockRuleService::create(&ctx, rule_type, target, scope, reason) {
        Ok(rule) => {
            match format {
                OutputFormat::Json | OutputFormat::Jsonl => print_json(&rule),
                OutputFormat::Text | OutputFormat::Table => {
                    if !quiet {
                        println!("added block rule {} ({})", rule.id, rule.target);
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn remove(format: OutputFormat, quiet: bool, block_rule_id: String) -> ExitCode {
    let id = match parse_block_rule_id(format, quiet, &block_rule_id) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match BlockRuleService::remove(&ctx, id) {
        Ok(()) => {
            if !quiet && format == OutputFormat::Text {
                println!("removed block rule {block_rule_id}");
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}

pub fn set_enabled(
    format: OutputFormat,
    quiet: bool,
    block_rule_id: String,
    enabled: bool,
) -> ExitCode {
    let id = match parse_block_rule_id(format, quiet, &block_rule_id) {
        Ok(id) => id,
        Err(code) => return code,
    };
    let ctx = match open_context(format, quiet) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };
    match BlockRuleService::set_enabled(&ctx, id, enabled) {
        Ok(()) => {
            if !quiet && format == OutputFormat::Text {
                let verb = if enabled { "enabled" } else { "disabled" };
                println!("{verb} block rule {block_rule_id}");
            }
            ExitCode::Success
        }
        Err(err) => report_and_exit(format, quiet, &err),
    }
}
