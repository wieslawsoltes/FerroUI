//! Compile ES modules to the System.register ABI implemented by the V8 host.
//! SWC owns live-export updates, re-exports, cycles and top-level await lowering.
use swc_common::{sync::Lrc, FileName, Globals, Mark, SourceMap, GLOBALS};
use swc_ecma_ast::{EsVersion, Pass, Program};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, Syntax};
use swc_ecma_transforms_base::{fixer::fixer, hygiene::hygiene, resolver};
use swc_ecma_transforms_module::{path::Resolver, system_js};

pub(super) fn compile(url: &str, source: &str) -> Result<String, String> {
    GLOBALS.set(&Globals::new(), || {
        let map = Lrc::new(SourceMap::default());
        let file = map.new_source_file(Lrc::new(FileName::Custom(url.into())), source.to_owned());
        let lexer = Lexer::new(
            Syntax::Es(EsSyntax::default()),
            EsVersion::EsNext,
            (&*file).into(),
            None,
        );
        let mut parser = Parser::new_from(lexer);
        let module = parser
            .parse_module()
            .map_err(|e| e.kind().msg().into_owned())?;
        if let Some(error) = parser.take_errors().into_iter().next() {
            return Err(error.kind().msg().into_owned());
        }
        let mut program = Program::Module(module);
        let unresolved = Mark::new();
        resolver(unresolved, Mark::new(), false).process(&mut program);
        system_js(Resolver::Default, unresolved, Default::default()).process(&mut program);
        hygiene().process(&mut program);
        fixer(None).process(&mut program);
        Ok(swc_ecma_codegen::to_code_default(map, None, &program))
    })
}
