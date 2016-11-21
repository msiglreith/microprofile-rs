
extern crate microprofile;
extern crate glutin;
extern crate libc;

use microprofile::Color;

fn main() {
    let window = glutin::Window::new().unwrap();

    let profiler = microprofile::Profiler::global();

    unsafe { window.make_current() };

    profiler.begin_thread("main");
    
    profiler.enable_all_groups(true);
    profiler.enable_all_meta_counters(true);
    
    #[cfg(feature = "gl")]
    profiler.init_gl(|symbol| { window.get_proc_address(symbol) as *mut _});

    profiler.begin_context_switch_trace();

    let render_category = profiler.define_category("render");
    let cull_group = render_category.define_group("cull", Color(40, 0, 250));
    let mut event_scope = cull_group.get_cpu_scope("events", Color(250, 0, 100));
    let mut test_scope = cull_group.get_cpu_scope("events2", Color(200, 200, 10));

    let mut main_counter = profiler.define_counter("main");
    main_counter += 10;

    loop {
        profiler.flip();

        {
            let cull_scoped = microprofile::SmartScope::new(&mut event_scope);
            for event in window.poll_events() {
                match event {
                    glutin::Event::Closed => break,
                    _ => ()
                }
            }
        }
        
        window.swap_buffers();
    }
}