// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod aptos_debug_natives;
mod built_package;
pub use built_package::*;

use crate::common::utils::{create_dir_if_not_exist, dir_default_to_current};
use crate::{
    common::{
        types::{
            load_account_arg, AccountAddressWrapper, CliError, CliTypedResult, MovePackageDir,
            PromptOptions, TransactionOptions, TransactionSummary,
        },
        utils::check_if_file_exists,
    },
    CliCommand, CliResult,
};
use aptos_gas::NativeGasParameters;
use aptos_module_verifier::module_init::verify_module_init_function;
use aptos_rest_client::aptos_api_types::MoveType;
use aptos_types::transaction::{ModuleBundle, ScriptFunction, TransactionPayload};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use framework::natives::code::UpgradePolicy;
use move_deps::move_cli::base::test::UnitTestResult;
use move_deps::{
    move_cli,
    move_core_types::{
        identifier::Identifier,
        language_storage::{ModuleId, TypeTag},
    },
    move_package::{
        compilation::compiled_package::CompiledPackage,
        source_package::layout::SourcePackageLayout, BuildConfig,
    },
    move_prover,
    move_unit_test::UnitTestingConfig,
};
use std::{
    collections::BTreeMap,
    convert::TryFrom,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::task;

/// CLI tool for performing Move tasks
///
#[derive(Subcommand)]
pub enum MoveTool {
    Compile(CompilePackage),
    Init(InitPackage),
    Publish(PublishPackage),
    Run(RunFunction),
    Test(TestPackage),
    Prove(ProvePackage),
}

impl MoveTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MoveTool::Compile(tool) => tool.execute_serialized().await,
            MoveTool::Init(tool) => tool.execute_serialized_success().await,
            MoveTool::Publish(tool) => tool.execute_serialized().await,
            MoveTool::Run(tool) => tool.execute_serialized().await,
            MoveTool::Test(tool) => tool.execute_serialized().await,
            MoveTool::Prove(tool) => tool.execute_serialized().await,
        }
    }
}

/// Creates a new Move package at the given location
#[derive(Parser)]
pub struct InitPackage {
    /// Name of the new move package
    #[clap(long)]
    name: String,
    /// Path to create the new move package
    #[clap(long, parse(from_os_str))]
    package_dir: Option<PathBuf>,
    /// Named addresses for the move binary
    ///
    /// Example: alice=0x1234, bob=0x5678
    ///
    /// Note: This will fail if there are duplicates in the Move.toml file remove those first.
    #[clap(long, parse(try_from_str = crate::common::utils::parse_map), default_value = "")]
    named_addresses: BTreeMap<String, AccountAddressWrapper>,
    #[clap(flatten)]
    prompt_options: PromptOptions,
}

#[async_trait]
impl CliCommand<()> for InitPackage {
    fn command_name(&self) -> &'static str {
        "InitPackage"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let package_dir = dir_default_to_current(self.package_dir.clone())?;
        let move_toml = package_dir.join(SourcePackageLayout::Manifest.path());
        check_if_file_exists(move_toml.as_path(), self.prompt_options)?;
        create_dir_if_not_exist(
            package_dir
                .join(SourcePackageLayout::Sources.path())
                .as_path(),
        )?;
        let mut w = std::fs::File::create(move_toml.as_path()).map_err(|err| {
            CliError::UnexpectedError(format!(
                "Failed to create {}: {}",
                package_dir.join(Path::new("Move.toml")).display(),
                err
            ))
        })?;

        let addresses: BTreeMap<String, String> = self
            .named_addresses
            .clone()
            .into_iter()
            .map(|(key, value)| (key, value.account_address.to_hex_literal()))
            .collect();

        // TODO: Support Git as default when Github credentials are properly handled from GH CLI
        writeln!(
            &mut w,
            "[package]
name = \"{}\"
version = \"0.0.0\"

[dependencies]
AptosFramework = {{ git = \"https://github.com/aptos-labs/aptos-core.git\", subdir = \"aptos-move/framework/aptos-framework/\", rev = \"devnet\" }}

[addresses]
{}
",
            self.name,
            toml::to_string(&addresses).map_err(|err| CliError::UnexpectedError(err.to_string()))?
        )
        .map_err(|err| {
            CliError::UnexpectedError(format!(
                "Failed to write {:?}: {}",
                package_dir.join(Path::new("Move.toml")),
                err
            ))
        })
    }
}

/// Compiles a package and returns the [`ModuleId`]s
#[derive(Parser)]
pub struct CompilePackage {
    #[clap(flatten)]
    move_options: MovePackageDir,
}

#[async_trait]
impl CliCommand<Vec<String>> for CompilePackage {
    fn command_name(&self) -> &'static str {
        "CompilePackage"
    }

    async fn execute(self) -> CliTypedResult<Vec<String>> {
        let build_config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses(),
            generate_abis: true,
            generate_docs: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let compiled_package = compile_move(
            build_config,
            self.move_options.get_package_path()?.as_path(),
        )?;
        let mut ids = Vec::new();
        for &module in compiled_package.root_modules_map().iter_modules().iter() {
            verify_module_init_function(module)
                .map_err(|e| CliError::MoveCompilationError(e.to_string()))?;
            ids.push(module.self_id().to_string());
        }
        Ok(ids)
    }
}

/// Run Move unit tests against a package path
#[derive(Parser)]
pub struct TestPackage {
    #[clap(flatten)]
    move_options: MovePackageDir,

    /// A filter string to determine which unit tests to run
    #[clap(long)]
    pub filter: Option<String>,
}

#[async_trait]
impl CliCommand<&'static str> for TestPackage {
    fn command_name(&self) -> &'static str {
        "TestPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses(),
            test_mode: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let result = move_cli::base::test::run_move_unit_tests(
            self.move_options.get_package_path()?.as_path(),
            config,
            UnitTestingConfig {
                filter: self.filter,
                ..UnitTestingConfig::default_with_bound(Some(100_000))
            },
            // TODO(Gas): we may want to switch to non-zero costs in the future
            aptos_debug_natives::aptos_debug_natives(NativeGasParameters::zeros()),
            false,
            &mut std::io::stdout(),
        )
        .map_err(|err| CliError::UnexpectedError(err.to_string()))?;

        // TODO: commit back up to the move repo
        match result {
            UnitTestResult::Success => Ok("Success"),
            UnitTestResult::Failure => Err(CliError::MoveTestError),
        }
    }
}

/// Prove the Move package at the package path
#[derive(Parser)]
pub struct ProvePackage {
    #[clap(flatten)]
    move_options: MovePackageDir,

    /// A filter string to determine which unit tests to run
    #[clap(long)]
    pub filter: Option<String>,
}

#[async_trait]
impl CliCommand<&'static str> for ProvePackage {
    fn command_name(&self) -> &'static str {
        "ProvePackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses(),
            test_mode: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let result = task::spawn_blocking(move || {
            move_cli::base::prove::run_move_prover(
                config,
                self.move_options.get_package_path()?.as_path(),
                &self.filter,
                true,
                move_prover::cli::Options::default(),
            )
        })
        .await
        .map_err(|err| CliError::UnexpectedError(err.to_string()))?;

        match result {
            Ok(_) => Ok("Success"),
            Err(_) => Err(CliError::MoveProverError),
        }
    }
}

/// Compiles a Move package dir, and returns the compiled modules.
fn compile_move(build_config: BuildConfig, package_dir: &Path) -> CliTypedResult<CompiledPackage> {
    // TODO: Add caching
    build_config
        .compile_package(package_dir, &mut Vec::new())
        .map_err(|err| CliError::MoveCompilationError(err.to_string()))
}

/// Publishes the modules in a Move package
#[derive(Parser)]
pub struct PublishPackage {
    #[clap(flatten)]
    move_options: MovePackageDir,
    #[clap(flatten)]
    txn_options: TransactionOptions,
    /// Whether to use the new publishing flow.
    #[clap(long)]
    new_flow: bool,
    /// The upgrade policy used for the published package (new flow only). One of
    /// `arbitrary`, `compatible`, or `immutable`. Defaults to `compatible`.
    #[clap(long)]
    upgrade_policy: Option<UpgradePolicy>,
}

#[async_trait]
impl CliCommand<TransactionSummary> for PublishPackage {
    fn command_name(&self) -> &'static str {
        "PublishPackage"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let PublishPackage {
            move_options,
            txn_options,
            new_flow,
            upgrade_policy,
        } = self;
        let package = BuiltPackage::build(move_options, true, true)?;
        let compiled_units = package.extract_code();
        if !new_flow {
            if upgrade_policy.is_some() {
                return Err(CliError::CommandArgumentError(
                    "`--upgrade-policy` can only be used with the `--new-flow` option".to_owned(),
                ));
            }
            // Send the compiled module using a module bundle
            txn_options
                .submit_transaction(TransactionPayload::ModuleBundle(ModuleBundle::new(
                    compiled_units,
                )))
                .await
                .map(TransactionSummary::from)
        } else {
            // Send the compiled module and metadata using the code::publish_package_txn.
            let metadata =
                package.extract_metadata(upgrade_policy.unwrap_or_else(UpgradePolicy::compat))?;
            let payload = aptos_transaction_builder::aptos_stdlib::code_publish_package_txn(
                bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
                compiled_units,
            );
            txn_options
                .submit_transaction(payload)
                .await
                .map(TransactionSummary::from)
        }
    }
}

/// Run a Move function
#[derive(Parser)]
pub struct RunFunction {
    #[clap(flatten)]
    txn_options: TransactionOptions,
    /// Function name as `<ADDRESS>::<MODULE_ID>::<FUNCTION_NAME>`
    ///
    /// Example: `0x842ed41fad9640a2ad08fdd7d3e4f7f505319aac7d67e1c0dd6a7cce8732c7e3::message::set_message`
    #[clap(long)]
    function_id: MemberId,
    /// Hex encoded arguments separated by spaces.
    ///
    /// Example: `0x01 0x02 0x03`
    #[clap(long, multiple_values = true)]
    args: Vec<ArgWithType>,
    /// TypeTag arguments separated by spaces.
    ///
    /// Example: `u8 u64 u128 bool address vector true false signer`
    #[clap(long, multiple_values = true)]
    type_args: Vec<MoveType>,
}

#[async_trait]
impl CliCommand<TransactionSummary> for RunFunction {
    fn command_name(&self) -> &'static str {
        "RunFunction"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let args: Vec<Vec<u8>> = self
            .args
            .iter()
            .map(|arg_with_type| arg_with_type.arg.clone())
            .collect();
        let mut type_args: Vec<TypeTag> = Vec::new();

        // These TypeArgs are used for generics
        for type_arg in self.type_args.iter().cloned() {
            let type_tag = TypeTag::try_from(type_arg)
                .map_err(|err| CliError::UnableToParse("--type-args", err.to_string()))?;
            type_args.push(type_tag)
        }

        self.txn_options
            .submit_transaction(TransactionPayload::ScriptFunction(ScriptFunction::new(
                self.function_id.module_id.clone(),
                self.function_id.member_id.clone(),
                type_args,
                args,
            )))
            .await
            .map(TransactionSummary::from)
    }
}

#[derive(Clone, Debug)]
enum FunctionArgType {
    Address,
    Bool,
    Hex,
    String,
    U8,
    U64,
    U128,
}

impl FunctionArgType {
    fn parse_arg(&self, arg: &str) -> CliTypedResult<Vec<u8>> {
        match self {
            FunctionArgType::Address => bcs::to_bytes(
                &load_account_arg(arg)
                    .map_err(|err| CliError::UnableToParse("address", err.to_string()))?,
            ),
            FunctionArgType::Bool => bcs::to_bytes(
                &bool::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("bool", err.to_string()))?,
            ),
            FunctionArgType::Hex => bcs::to_bytes(
                &hex::decode(arg).map_err(|err| CliError::UnableToParse("hex", err.to_string()))?,
            ),
            FunctionArgType::String => bcs::to_bytes(arg),
            FunctionArgType::U8 => bcs::to_bytes(
                &u8::from_str(arg).map_err(|err| CliError::UnableToParse("u8", err.to_string()))?,
            ),
            FunctionArgType::U64 => bcs::to_bytes(
                &u64::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u64", err.to_string()))?,
            ),
            FunctionArgType::U128 => bcs::to_bytes(
                &u128::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u128", err.to_string()))?,
            ),
        }
        .map_err(|err| CliError::BCS("arg", err))
    }
}

impl FromStr for FunctionArgType {
    type Err = CliError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "address" => Ok(FunctionArgType::Address),
            "bool" => Ok(FunctionArgType::Bool),
            "hex" => Ok(FunctionArgType::Hex),
            "string" => Ok(FunctionArgType::String),
            "u8" => Ok(FunctionArgType::U8),
            "u64" => Ok(FunctionArgType::U64),
            "u128" => Ok(FunctionArgType::U128),
            str => Err(CliError::CommandArgumentError(format!("Invalid arg type '{}'.  Must be one of: ['address','bool','hex','string','u8','u64','u128']", str))),
        }
    }
}

/// A parseable arg with a type separated by a colon
pub struct ArgWithType {
    _ty: FunctionArgType,
    arg: Vec<u8>,
}

impl FromStr for ArgWithType {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(CliError::CommandArgumentError(
                "Arguments must be pairs of <type>:<arg> e.g. bool:true".to_string(),
            ));
        }

        let ty = FunctionArgType::from_str(parts.first().unwrap())?;
        let arg = parts.last().unwrap();
        let arg = ty.parse_arg(arg)?;

        Ok(ArgWithType { _ty: ty, arg })
    }
}

/// Identifier of a module member (function or struct).
#[derive(Debug, Clone)]
pub struct MemberId {
    pub module_id: ModuleId,
    pub member_id: Identifier,
}

fn parse_member_id(function_id: &str) -> CliTypedResult<MemberId> {
    let ids: Vec<&str> = function_id.split_terminator("::").collect();
    if ids.len() != 3 {
        return Err(CliError::CommandArgumentError(
            "FunctionId is not well formed.  Must be of the form <address>::<module>::<function>"
                .to_string(),
        ));
    }
    let address = load_account_arg(ids.get(0).unwrap())?;
    let module = Identifier::from_str(ids.get(1).unwrap())
        .map_err(|err| CliError::UnableToParse("Module Name", err.to_string()))?;
    let member_id = Identifier::from_str(ids.get(2).unwrap())
        .map_err(|err| CliError::UnableToParse("Member Name", err.to_string()))?;
    let module_id = ModuleId::new(address, module);
    Ok(MemberId {
        module_id,
        member_id,
    })
}

impl FromStr for MemberId {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_member_id(s)
    }
}
