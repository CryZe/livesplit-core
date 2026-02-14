cfg_if::cfg_if! {
    if #[cfg(any(feature = "software-rendering", feature = "software-rendering-vello"))] {
        use {
            criterion::{criterion_group, criterion_main, Criterion},
            livesplit_core::{
                layout::{self, Layout},
                rendering,
                run::parser::livesplit,
                settings::ImageCache,
                Lang, Run, Segment, TimeSpan, Timer, TimingMethod,
            },
            std::fs,
        };

        criterion_main!(benches);
        criterion_group!(benches, default, subsplits_layout, default_1080p, subsplits_layout_1080p);

        fn default(c: &mut Criterion) {
            let mut run = create_run(&["A", "B", "C", "D"]);
            run.set_game_name("Some Game Name");
            run.set_category_name("Some Category Name");
            run.set_attempt_count(1337);
            let mut timer = Timer::new(run).unwrap();
            let mut layout = Layout::default_layout(Lang::English);
            let mut image_cache = ImageCache::new();

            start_run(&mut timer);
            make_progress_run_with_splits_opt(&mut timer, &[Some(5.0), None, Some(10.0)]);

            let state = layout.state(&mut image_cache, &timer.snapshot(), Lang::English);
            #[cfg(feature = "software-rendering")]
            {
                let mut renderer = rendering::software::Renderer::new();
                bench_render(c, "Software Rendering (TinySkia, Default)", || {
                    renderer.render(&state, &image_cache, [300, 500]);
                });
            }

            #[cfg(feature = "software-rendering-vello")]
            {
                let mut renderer = rendering::software_vello::Renderer::new();
                bench_render(c, "Software Rendering (Vello, Default)", || {
                    renderer.render(&state, &image_cache, [300, 500]);
                });
            }
        }

        fn subsplits_layout(c: &mut Criterion) {
            let run = lss("tests/run_files/Celeste - Any% (1.2.1.5).lss");
            let mut timer = Timer::new(run).unwrap();
            let mut layout = lsl("tests/layout_files/subsplits.lsl");
            let mut image_cache = ImageCache::new();

            start_run(&mut timer);
            make_progress_run_with_splits_opt(&mut timer, &[Some(10.0), None, Some(20.0), Some(55.0)]);

            let state = layout.state(&mut image_cache, &timer.snapshot(), Lang::English);
            #[cfg(feature = "software-rendering")]
            {
                let mut renderer = rendering::software::Renderer::new();
                bench_render(
                    c,
                    "Software Rendering (TinySkia, Subsplits Layout)",
                    || {
                        renderer.render(&state, &image_cache, [300, 800]);
                    },
                );
            }

            #[cfg(feature = "software-rendering-vello")]
            {
                let mut renderer = rendering::software_vello::Renderer::new();
                bench_render(
                    c,
                    "Software Rendering (Vello, Subsplits Layout)",
                    || {
                        renderer.render(&state, &image_cache, [300, 800]);
                    },
                );
            }
        }

        fn default_1080p(c: &mut Criterion) {
            let mut run = create_run(&["A", "B", "C", "D"]);
            run.set_game_name("Some Game Name");
            run.set_category_name("Some Category Name");
            run.set_attempt_count(1337);
            let mut timer = Timer::new(run).unwrap();
            let mut layout = Layout::default_layout(Lang::English);
            let mut image_cache = ImageCache::new();

            start_run(&mut timer);
            make_progress_run_with_splits_opt(&mut timer, &[Some(5.0), None, Some(10.0)]);

            let state = layout.state(&mut image_cache, &timer.snapshot(), Lang::English);
            #[cfg(feature = "software-rendering")]
            {
                let mut renderer = rendering::software::Renderer::new();
                bench_render(c, "Software Rendering (TinySkia, Default 1080p)", || {
                    renderer.render(&state, &image_cache, [1920, 1080]);
                });
            }

            #[cfg(feature = "software-rendering-vello")]
            {
                let mut renderer = rendering::software_vello::Renderer::new();
                bench_render(c, "Software Rendering (Vello, Default 1080p)", || {
                    renderer.render(&state, &image_cache, [1920, 1080]);
                });
            }
        }

        fn subsplits_layout_1080p(c: &mut Criterion) {
            let run = lss("tests/run_files/Celeste - Any% (1.2.1.5).lss");
            let mut timer = Timer::new(run).unwrap();
            let mut layout = lsl("tests/layout_files/subsplits.lsl");
            let mut image_cache = ImageCache::new();

            start_run(&mut timer);
            make_progress_run_with_splits_opt(&mut timer, &[Some(10.0), None, Some(20.0), Some(55.0)]);

            let state = layout.state(&mut image_cache, &timer.snapshot(), Lang::English);
            #[cfg(feature = "software-rendering")]
            {
                let mut renderer = rendering::software::Renderer::new();
                bench_render(
                    c,
                    "Software Rendering (TinySkia, Subsplits 1080p)",
                    || {
                        renderer.render(&state, &image_cache, [1920, 1080]);
                    },
                );
            }

            #[cfg(feature = "software-rendering-vello")]
            {
                let mut renderer = rendering::software_vello::Renderer::new();
                bench_render(
                    c,
                    "Software Rendering (Vello, Subsplits 1080p)",
                    || {
                        renderer.render(&state, &image_cache, [1920, 1080]);
                    },
                );
            }
        }

        fn bench_render(c: &mut Criterion, name: &str, mut render: impl FnMut()) {
            c.bench_function(name, move |b| b.iter(&mut render));
        }

        fn file(path: &str) -> String {
            fs::read_to_string(path).unwrap()
        }

        fn lss(path: &str) -> Run {
            livesplit::parse(&file(path)).unwrap()
        }

        fn lsl(path: &str) -> Layout {
            layout::parser::parse(&file(path)).unwrap()
        }

        fn create_run(names: &[&str]) -> Run {
            let mut run = Run::new();
            for &name in names {
                run.push_segment(Segment::new(name));
            }
            run
        }

        fn start_run(timer: &mut Timer) {
            timer.set_current_timing_method(TimingMethod::GameTime);
            timer.start().unwrap();
            timer.initialize_game_time().unwrap();
            timer.pause_game_time().unwrap();
            timer.set_game_time(TimeSpan::zero()).unwrap();
        }

        fn make_progress_run_with_splits_opt(timer: &mut Timer, splits: &[Option<f64>]) {
            for &split in splits {
                if let Some(split) = split {
                    timer.set_game_time(TimeSpan::from_seconds(split)).unwrap();
                    timer.split().unwrap();
                } else {
                    timer.skip_split().unwrap();
                }
            }
        }
    } else {
        fn main() {}
    }
}
