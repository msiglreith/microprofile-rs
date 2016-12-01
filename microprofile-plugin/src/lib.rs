#![feature(plugin_registrar, quote, rustc_private)]

extern crate syntax;
extern crate rustc_plugin;
extern crate microprofile;

use rustc_plugin::registry::Registry;
use syntax::ast::{Block, Expr, ExprKind, Item, ItemKind, Mac, MetaItem};
use syntax::fold::{self, Folder};
use syntax::symbol::Symbol;
use syntax::ext::base::{Annotatable, ExtCtxt, SyntaxExtension};
use syntax::ext::build::AstBuilder;
use syntax::codemap::{DUMMY_SP, Span};
use syntax::ptr::P;
use syntax::util::small_vector::SmallVector;

pub fn expand_scope_cpu(ecx: &mut ExtCtxt, _span: Span, _meta_item: &MetaItem, item: Annotatable) -> Annotatable {
    match item {
        Annotatable::Item(item) => {
            Annotatable::Item(ScopeFolder { ecx: ecx, symbol: item.ident.name }.fold_item(item).expect_one("expected one item"))
        }
        Annotatable::TraitItem(item) => {
            Annotatable::TraitItem(item.map(|i| ScopeFolder { ecx: ecx, symbol: i.ident.name }.fold_trait_item(i).expect_one("expected one trait item")))
        }
        Annotatable::ImplItem(item) => {
            Annotatable::ImplItem(item.map(|i| ScopeFolder { ecx: ecx, symbol: i.ident.name }.fold_impl_item(i).expect_one("expected one impl item")))
        }
    }
}

struct ScopeFolder<'a, 'ecx: 'a> {
    symbol: Symbol,
    ecx: &'a mut ExtCtxt<'ecx>,
}

impl<'a, 'ecx> Folder for ScopeFolder<'a, 'ecx> {
    fn fold_item_simple(&mut self, i: Item) -> Item {
        if let ItemKind::Mac(_) = i.node {
            return i;
        } else {
            self.symbol = i.ident.name;
            fold::noop_fold_item_simple(i, self)
        }
    }

    fn fold_block(&mut self, block: P<Block>) -> P<Block> {
        block.map(|block| {
            let name = self.ecx.expr_str(DUMMY_SP, self.symbol);
            println!("{:?}", name);
            quote_block!(self.ecx, {
                use ::microprofile::Scope;
                let profiler = ::microprofile::Profiler::global();
                let category = profiler.define_category("profile");
                let group = category.define_group("trace", ::microprofile::Color(40, 0, 250));
                let mut scope = group.get_cpu_scope($name, ::microprofile::Color(250, 0, 100));
                scope.enter();
                println!("begin {:?}", $name);
                let r = $block;
                println!("end {:?}", $name);
                scope.leave();
                r
            }).unwrap()
        })
    }
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_syntax_extension(Symbol::intern("profile"), SyntaxExtension::MultiModifier(Box::new(expand_scope_cpu)));
}
