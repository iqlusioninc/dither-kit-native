use dither_gtk::renderer::{DATA_WIDTH, blit_rect, pack_program};

#[test]
fn programs_use_fixed_width_rows_and_zero_padding() {
    let mut values = vec![3.5; 1025];
    values[0] = 0.0;
    let (data, rows) = pack_program(&values).unwrap();
    assert_eq!(DATA_WIDTH, 1024);
    assert_eq!(rows, 2);
    assert_eq!(&data[..1025], &values);
    assert!(data[1025..].iter().all(|&x| x == 0.0));
    assert!(pack_program(&[]).is_err());
}

#[test]
fn blit_is_integer_scaled_and_anchored_at_top_left() {
    assert_eq!(blit_rect(30, 20, 3, 2, 130), [0, 10, 180, 130]);
}

#[test]
fn malformed_ops_fail_before_reaching_the_shader() {
    assert!(pack_program(&[3.0, 1.0, 0.0, 0.0]).is_err());
    assert!(pack_program(&[4.0, 1.0, 4.0]).is_err());
    assert!(pack_program(&[0.0, f32::NAN]).is_err());
    assert!(pack_program(&[0.0]).is_ok());
}
