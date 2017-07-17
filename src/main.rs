extern crate image;
extern crate ord_subset;
extern crate rayon;

use image::GenericImage;
use ord_subset::OrdSubsetIterExt;

use std::env::args;
use std::path::Path;
use std::fs::create_dir_all;

// A 1D line of pixels
type Line = Vec<image::Rgb<u8>>;

// Get the average difference between two lines of pixels
fn average_difference(old: &Line, new: &Line) -> f64 {
    let mut value = 0.0;

    // Sum up the difference for each channel of each pixel
    for index in 0 .. old.len() {
        for channel in 0 .. 3 {
            value += (old[index][channel] as f64 - new[index][channel] as f64).abs();
        }
    }

    // Divide by number of pixels and then by the number of channels
    value / old.len() as f64 / 3.0
}

fn difference_horizontal(image: &image::RgbImage) -> (u32, f64) {
    // Create vecs for the old and new lines of pixels
    let mut old: Line = Vec::with_capacity(image.width() as usize);
    let mut new: Line = vec![image::Rgb {data: [0; 3]}; image.width() as usize];
    let mut values = Vec::with_capacity(image.height() - 1);

    // Loop through the pixels T -> B, L -> R
    for (x, _, pixel) in image.enumerate_pixels() {
        // set a pixel in the line
        new[x as usize] = *pixel;

        if x == image.width() - 1 {
            // If the pixel is at the end of the line and the old line of pixels is not empty, add the difference
            if !old.is_empty() {
                values.push(average_difference(&old, &new));
            }
            // Copy the new line to the old line
            old = new.clone();
        }
    }

    // Find the maximum difference and its place to cut
    values.iter()
        .enumerate()
        .ord_subset_max_by_key(|&(_, value)| value)
        .map(|(x, y)| (x as u32 + 1, *y))
        .unwrap()
}

fn difference_vertical(image: &image::RgbImage) -> (u32, f64) {
    // Create vecs for the old and new lines of pixels
    let mut old: Line = Vec::with_capacity(image.height() as usize);
    let mut new: Line = vec![image::Rgb {data: [0; 3]}; image.height() as usize];
    let mut values = Vec::with_capacity(image.width() - 1);

    // Loop through the pixels L -> R, T -> B
    for x in 0 .. image.width() {
        for y in 0 .. image.height() {
            // set a pixel in the line
            new[y as usize] = *image.get_pixel(x, y);

            if y == image.height() - 1 {
                // If the pixel is at the end of the line and the old line of pixels is not empty, add the difference
                if !old.is_empty() {
                    values.push(average_difference(&old, &new));
                }
                // Copy the new line to the old line
                old = new.clone();
            }
        }
    }

    // Find the maximum difference and its place to cut
    values.iter()
        .enumerate()
        .ord_subset_max_by_key(|&(_, value)| value)
        .map(|(x, y)| (x as u32 + 1, *y))
        .unwrap()
}

fn guillotine(mut image: image::RgbImage, threshold: f64, min_size: u32) -> Vec<image::RgbImage> {
    let (width, height) = image.dimensions();

    // If the image is too small, don't return anything
    if width < min_size || height < min_size {
        return Vec::new();
    }

    // Get the maximum differences and the place to cut 
    let (h_index, h_max) = difference_horizontal(&image);
    let (v_index, v_max) = difference_vertical(&image);

    let horizontal = h_max > v_max;
    let max = if horizontal { h_max } else { v_max };
    let cut = max > threshold;

    println!("Cut: {}, Horizontal: {}, Max: {}", cut, horizontal, max);

    // If the image won't be cut, return it
    if !cut {
        return vec![image];
    }

    // Create the two sub images
    let (sub_a, sub_b) = if horizontal {(
        image.sub_image(0, 0, width, h_index).to_image(),
        image.sub_image(0, h_index, width, height - h_index).to_image()
    )} else {(
        image.sub_image(0, 0, v_index, height).to_image(),
        image.sub_image(v_index, 0, width - v_index, height).to_image()
    )};

    // Guillotine the two sub images in parallel
    let (mut g_a, mut g_b) = rayon::join(
        || guillotine(sub_a, threshold, min_size),
        || guillotine(sub_b, threshold, min_size)
    );

    // Move the guillotined images into images
    let mut images = Vec::new();
    images.append(&mut g_a);
    images.append(&mut g_b);

    // And return them
    images
}

fn main() {
    for arg in args().skip(1) {
        let path = Path::new(&arg);
        // Load the image
        let image = image::open(path).unwrap().to_rgb();
        // Autoguillotine the image
        let images = guillotine(image, 30.0, 100);
        // Get the filename without the extension as the dir
        let dir = path.parent().unwrap().join(path.file_stem().unwrap());
        // Create the dir
        create_dir_all(&dir).unwrap();

        // Save the images
        for (i, image) in images.iter().enumerate() {
            let path = dir.join(format!("{}.png", i));
            println!("Saving {}...", path.display());
            image.save(path).unwrap();
        }
    }
}