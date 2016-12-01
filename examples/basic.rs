
#![feature(custom_attribute)]
#![feature(plugin)]
#![plugin(microprofile_plugin)]

#![profile]

extern crate microprofile;
extern crate libc;

use microprofile::Color;
use microprofile::Scope;

fn test() {
    foo::a();
}

mod foo {
    pub fn a() {
    }
}

fn main() {
    let profiler = microprofile::Profiler::global();
    profiler.begin_thread("main");
    
    profiler.enable_all_groups(true);
    profiler.enable_all_meta_counters(true);

    profiler.begin_context_switch_trace();

    let render_category = profiler.define_category("render");
    let cull_group = render_category.define_group("cull", Color(40, 0, 250));
    let mut event_scope = cull_group.get_cpu_scope("events", Color(250, 0, 100));
    let mut test_scope = cull_group.get_cpu_scope("events2", Color(200, 200, 10));

    let mut main_counter = profiler.define_counter("main");
    main_counter += 10;

    'main: loop {
        test();
    }
}