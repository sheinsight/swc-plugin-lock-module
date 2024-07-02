use serde::Deserialize;
use swc_core::ecma::{
    ast::{ImportDecl, Program},
    transforms::testing::test_inline,
    visit::{as_folder, FoldWith, VisitMut},
};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

#[derive(Debug, Clone, Deserialize)]
pub struct TransformModuleVisitorConfig {
    pub enable: bool,
    pub source: String,
    pub target: String,
}

pub struct TransformModuleVisitor {
    pub config: Option<TransformModuleVisitorConfig>,
}

impl VisitMut for TransformModuleVisitor {
    // https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html
    fn visit_mut_import_decl(&mut self, import_decl: &mut ImportDecl) {
        if let Some(config) = &self.config {
            if config.enable {
                let src_value = import_decl.src.value.as_str();
                if import_decl.specifiers.is_empty() {
                    src_value.starts_with(&config.source.as_str());
                    let source = src_value.replace(&config.source, &config.target);
                    import_decl.src = Box::new(source.into());
                }
            }
        }
    }
}

/// An example plugin function with macro support.
/// `plugin_transform` macro interop pointers into deserialized structs, as well
/// as returning ptr back to host.
///
/// It is possible to opt out from macro by writing transform fn manually
/// if plugin need to handle low-level ptr directly via
/// `__transform_plugin_process_impl(
///     ast_ptr: *const u8, ast_ptr_len: i32,
///     unresolved_mark: u32, should_enable_comments_proxy: i32) ->
///     i32 /*  0 for success, fail otherwise.
///             Note this is only for internal pointer interop result,
///             not actual transform result */`
///
/// This requires manual handling of serialization / deserialization from ptrs.
/// Refer swc_plugin_macro to see how does it work internally.
#[plugin_transform]
pub fn process_transform(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let empty_config = TransformModuleVisitorConfig {
        enable: false,
        source: String::from(""),
        target: String::from(""),
    };

    let config = metadata
        .get_transform_plugin_config()
        .and_then(|config_str| {
            serde_json::from_str::<TransformModuleVisitorConfig>(config_str.as_str()).ok()
        })
        .unwrap_or_else(|| empty_config);

    program.fold_with(&mut as_folder(TransformModuleVisitor {
        config: Some(config),
    }))
}

// An example to test plugin transform.
// Recommended strategy to test plugin's transform is verify
// the Visitor's behavior, instead of trying to run `process_transform` with mocks
// unless explicitly required to do so.
test_inline!(
    Default::default(),
    |_| as_folder(TransformModuleVisitor { config: None }),
    boo,
    // Input codes
    r#"console.log("transform");"#,
    // Output codes after transformed with plugin
    r#"console.log("transform");"#
);

test_inline!(
    Default::default(),
    |_| as_folder(TransformModuleVisitor {
        config: Some(TransformModuleVisitorConfig {
            enable: true,
            source: String::from("a"),
            target: String::from("x"),
        })
    }),
    boo1,
    // Input codes
    r#"
        import "a/b"
    "#,
    // Output codes after transformed with plugin
    r#"
        import "x/b"
    "#
);
