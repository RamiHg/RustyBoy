use super::image::*;
use super::integration::*;
use super::*;

use crate::cart;
use crate::gpu;
use crate::gpu::options::Options;

use itertools::iproduct;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use threadpool::ThreadPool;

#[test]
fn solve_best_gpu_params() {
    let targets_to_run = [
        "acceptance/ppu/intr_1_2_timing-GS",
        "acceptance/ppu/intr_2_0_timing",
        "acceptance/ppu/intr_2_mode0_timing",
        "acceptance/ppu/intr_2_mode3_timing",
        "acceptance/ppu/vblank_stat_intr-GS",
        "acceptance/ppu/intr_2_oam_ok_timing",
        "wilbert/lcdon_mode_timing",
        "wilbert/intr_0_timing",
        "wilbert/intr_1_timing",
        "wilbert/intr_2_timing",
        "wilbert/ly_new_frame-GS",
        "wilbert/ly00_01_mode0_2",
        "wilbert/ly00_mode2_3",
        "wilbert/ly00_mode3_0",
        "wilbert/ly143_144_145",
        "wilbert/ly143_144_152_153",
        "wilbert/ly143_144_mode0_1",
        "wilbert/ly143_144_mode3_0",
    ];

    let golden_images: Arc<Vec<Option<Vec<gpu::Pixel>>>> = Arc::new(
        targets_to_run
            .iter()
            .map(|target| {
                let golden_path = golden_image_path(target);
                assert!(golden_path.exists());
                Some(load_golden_image(golden_path))
            })
            .collect(),
    );

    let cart_contents: Arc<Vec<Vec<u8>>> = Arc::new(
        targets_to_run
            .iter()
            .map(|target| {
                let mut path = base_path_to("test_roms");
                path.push(format!("{}.gb", target));
                cart::read_file(path.to_str().unwrap())
            })
            .collect(),
    );

    //Found better candidate with 12 errors: Options { cycle_after_enable: 292, vblank_cycle: 4,
    // hblank_cycle: 0, oam_1_143_cycle: 0, oam_144_cycle: 0, oam_145_153_cycle: 0, oam_0_cycle: 0,
    // oam_0_vblank_cycle: 0, use_fetcher_initial_fetch: true }

    let cycle_after_enables = (74 * 4 + 0)..=(74 * 4 + 3);
    let vblank_cycles = [0].into_iter();
    let hblank_cycles = [-4].into_iter();
    let oam_1_143_cycles = [-4, 0, 4].into_iter();
    let oam_144_cycles = [-4, 0, 4].into_iter();
    let oam_145_152_cycles = [0, 4, 8].into_iter();
    let oam_0_cycles = [-4, 0, 4].into_iter();
    let oam_0_vblank_cycle_firsts = [0, 4].into_iter();
    let oam_0_vblank_cycle_seconds = [8, 12];

    #[derive(Copy, Clone)]
    struct Status {
        errors: i32,
        options: Options,
    }

    let min_status = Arc::new(Mutex::new(Status {
        errors: 10000,
        options: Options::default(),
    }));

    let pool = ThreadPool::new(8);

    for (
        cycle_after_enable,
        &vblank_cycle,
        &hblank_cycle,
        &oam_1_143_cycle,
        &oam_144_cycle,
        &oam_145_152_cycle,
        &oam_0_cycle,
        &oam_0_vblank_cycle_first,
    ) in iproduct!(
        cycle_after_enables,
        vblank_cycles,
        hblank_cycles,
        oam_1_143_cycles,
        oam_144_cycles,
        oam_145_152_cycles,
        oam_0_cycles,
        oam_0_vblank_cycle_firsts
    ) {
        for &use_fetcher_initial_fetch in &[true, false] {
            for &oam_0_vblank_cycle_second in oam_0_vblank_cycle_seconds.iter() {
                let min_status = Arc::clone(&min_status);
                let golden_images = Arc::clone(&golden_images);
                let cart_contents = Arc::clone(&cart_contents);
                pool.execute(move || {
                    let options = Options {
                        cycle_after_enable,
                        vblank_cycle,
                        hblank_cycle,
                        oam_1_143_cycle,
                        oam_144_cycle,
                        oam_145_152_cycle,
                        oam_0_cycle,
                        oam_0_vblank_cycle_first,
                        oam_0_vblank_cycle_second,
                        use_fetcher_initial_fetch,
                        // ..Default::default()
                    };

                    let mut num_errors = 0;
                    for (i, target) in targets_to_run.iter().enumerate() {
                        let cart = cart::from_file_contents(&cart_contents[i]);
                        num_errors +=
                            !run_target_with_options(target, cart, &golden_images[i], options)
                                as i32;
                        if num_errors > 0 && i <= 4 {
                            return;
                        }
                        // if i > 5 {
                        //     println!("Found viable candidate {:X?}", options);
                        // }
                        let status = min_status.lock().unwrap();
                        if num_errors >= status.errors {
                            // Give up on this option set.
                            break;
                        }
                    }
                    let mut status = min_status.lock().unwrap();
                    if num_errors < status.errors {
                        println!(
                            "Found better candidate with {} errors: {:?}",
                            num_errors, options
                        );
                        status.errors = num_errors;
                        status.options = options;
                    } else if status.errors == 0 {
                        return;
                    }
                });
            }
        }
    }

    while pool.queued_count() > 0 {
        thread::sleep_ms(10000);
        println!("{} more iterations remaining.", pool.queued_count());
    }

    pool.join();
    let status = min_status.lock().unwrap();
    println!(
        "Finished. {} errors with {:?}",
        status.errors, status.options
    );
}
