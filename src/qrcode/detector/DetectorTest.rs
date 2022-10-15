use std::collections::HashMap;

use crate::{
    common::{BitMatrix, DetectorRXingResult},
    qrcode::{
        decoder::decoder, decoder::ErrorCorrectionLevel, detector::Detector, encoder::encoder,
    },
};

#[test]
fn test_simple() {
    test_encode_decode("value");
}

#[test]
fn test_uri() {
    test_encode_decode("https://google.com");
}

#[test]
fn test_unicode() {
    test_encode_decode("\u{11D4}\u{1185}\u{11c2}");
}

fn test_encode_decode(value: &str) {
    for ec_level_v in 0..4 {
        let ec_level: ErrorCorrectionLevel =
            ErrorCorrectionLevel::forBits(ec_level_v).expect("must get level");
        let qr_code =
            encoder::encode_with_hints(value, ec_level, &HashMap::new()).expect("must encode");
        // dbg!(&qr_code.to_string());
        let byt_matrix = qr_code.getMatrix().as_ref().unwrap().clone();
        // dbg!(BitMatrix::from(byt_matrix.clone()).to_string());
        // let mut detector = Detector::new(make_larger(&byt_matrix.into(),5));
        let mut detector = Detector::new(byt_matrix.into());
        let detected_points = detector.detect().expect("must detect");
        let decoded = decoder::decode_bitmatrix(detected_points.getBits()).expect("must decode");
        assert_eq!(decoded.getText(), value);
    }
}

// Zooms a bit matrix so that each bit is factor x factor
fn make_larger(input: &BitMatrix, factor: u32) -> BitMatrix {
    let width = input.getWidth();
    let mut output = BitMatrix::with_single_dimension(width * factor);
    for inputY in 0..width {
        // for (int inputY = 0; inputY < width; inputY++) {
        for inputX in 0..width {
            // for (int inputX = 0; inputX < width; inputX++) {
            if input.get(inputX, inputY) {
                output
                    .setRegion(inputX * factor, inputY * factor, factor, factor)
                    .expect("region set should be ok");
            }
        }
    }
    return output;
}
