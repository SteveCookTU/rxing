/*
* Copyright 2016 Nu-book Inc.
* Copyright 2016 ZXing authors
* Copyright 2022 Axel Waggershauser
*/
// SPDX-License-Identifier: Apache-2.0

// Result Reader::decode(const BinaryBitmap& image) const
// {
// #if 1
// 	if (!_hints.isPure())
// 		return FirstOrDefault(decode(image, 1));
// #endif

// 	auto binImg = image.getBitMatrix();
// 	if (binImg == nullptr)
// 		return {};

// 	DetectorResult detectorResult;
// 	if (_hints.hasFormat(BarcodeFormat::QRCode))
// 		detectorResult = DetectPureQR(*binImg);
// 	if (_hints.hasFormat(BarcodeFormat::MicroQRCode) && !detectorResult.isValid())
// 		detectorResult = DetectPureMQR(*binImg);

// 	if (!detectorResult.isValid())
// 		return {};

// 	auto decoderResult = Decode(detectorResult.bits());
// 	auto position = detectorResult.position();

// 	return Result(std::move(decoderResult), std::move(position),
// 				  detectorResult.bits().width() < 21 ? BarcodeFormat::MicroQRCode : BarcodeFormat::QRCode);
// }

// void logFPSet(const FinderPatternSet& fps [[maybe_unused]])
// {
// #ifdef PRINT_DEBUG
// 	auto drawLine = [](PointF a, PointF b) {
// 		int steps = maxAbsComponent(b - a);
// 		PointF dir = bresenhamDirection(PointF(b - a));
// 		for (int i = 0; i < steps; ++i)
// 			log(centered(a + i * dir), 2);
// 	};

// 	drawLine(fps.bl, fps.tl);
// 	drawLine(fps.tl, fps.tr);
// 	drawLine(fps.tr, fps.bl);
// #endif
// }

// Results Reader::decode(const BinaryBitmap& image, int maxSymbols) const
// {
// 	auto binImg = image.getBitMatrix();
// 	if (binImg == nullptr)
// 		return {};

// #ifdef PRINT_DEBUG
// 	LogMatrixWriter lmw(log, *binImg, 5, "qr-log.pnm");
// #endif

// 	auto allFPs = FindFinderPatterns(*binImg, _hints.tryHarder());

// #ifdef PRINT_DEBUG
// 	printf("allFPs: %d\n", Size(allFPs));
// #endif

// 	std::vector<ConcentricPattern> usedFPs;
// 	Results results;

// 	if (_hints.hasFormat(BarcodeFormat::QRCode)) {
// 		auto allFPSets = GenerateFinderPatternSets(allFPs);
// 		for (const auto& fpSet : allFPSets) {
// 			if (Contains(usedFPs, fpSet.bl) || Contains(usedFPs, fpSet.tl) || Contains(usedFPs, fpSet.tr))
// 				continue;

// 			logFPSet(fpSet);

// 			auto detectorResult = SampleQR(*binImg, fpSet);
// 			if (detectorResult.isValid()) {
// 				auto decoderResult = Decode(detectorResult.bits());
// 				auto position = detectorResult.position();
// 				if (decoderResult.isValid()) {
// 					usedFPs.push_back(fpSet.bl);
// 					usedFPs.push_back(fpSet.tl);
// 					usedFPs.push_back(fpSet.tr);
// 				}
// 				if (decoderResult.isValid(_hints.returnErrors())) {
// 					results.emplace_back(std::move(decoderResult), std::move(position), BarcodeFormat::QRCode);
// 					if (maxSymbols && Size(results) == maxSymbols)
// 						break;
// 				}
// 			}
// 		}
// 	}

// 	if (_hints.hasFormat(BarcodeFormat::MicroQRCode) && !(maxSymbols && Size(results) == maxSymbols)) {
// 		for (const auto& fp : allFPs) {
// 			if (Contains(usedFPs, fp))
// 				continue;

// 			auto detectorResult = SampleMQR(*binImg, fp);
// 			if (detectorResult.isValid()) {
// 				auto decoderResult = Decode(detectorResult.bits());
// 				auto position = detectorResult.position();
// 				if (decoderResult.isValid(_hints.returnErrors())) {
// 					results.emplace_back(std::move(decoderResult), std::move(position), BarcodeFormat::MicroQRCode);
// 					if (maxSymbols && Size(results) == maxSymbols)
// 						break;
// 				}

// 			}
// 		}
// 	}

// 	return results;
// }

// } // namespace ZXing::QRCode

use crate::{
    common::{cpp_essentials::ConcentricPattern, DetectorRXingResult, HybridBinarizer},
    multi::MultipleBarcodeReader,
    BarcodeFormat, BinaryBitmap, DecodeHintType, DecodeHintValue, DecodingHintDictionary,
    Exceptions, RXingResult, Reader,
};

use super::{
    decoder::Decode,
    detector::{
        DetectPureMQR, DetectPureQR, FindFinderPatterns, GenerateFinderPatternSets, SampleMQR,
        SampleQR,
    },
};

use crate::qrcode::detector::QRCodeDetectorResult as DetectorResult;

#[derive(Default)]
pub struct QrReader;

impl Reader for QrReader {
    fn decode<B: crate::Binarizer>(
        &mut self,
        image: &mut crate::BinaryBitmap<B>,
    ) -> crate::common::Result<crate::RXingResult> {
        self.decode_with_hints(image, &DecodingHintDictionary::new())
    }

    fn decode_with_hints<B: crate::Binarizer>(
        &mut self,
        image: &mut crate::BinaryBitmap<B>,
        hints: &crate::DecodingHintDictionary,
    ) -> crate::common::Result<RXingResult> {
        // #if 1
        if !matches!(
            hints.get(&DecodeHintType::PURE_BARCODE),
            Some(DecodeHintValue::PureBarcode(true))
        )
        // if !matches!(Some(hints.get(&DecodeHintType::PURE_BARCODE)))
        // if (!_hints.isPure())
        {
            return Ok(self
                .decode_set_number_with_hints(image, hints, 1)?
                .first()
                .ok_or(Exceptions::NOT_FOUND)?
                .clone());
            // return FirstOrDefault(decode(image, 1));
        }
        // #endif

        let binImg = image.get_black_matrix(); //image.getBitMatrix();
                                               // if (binImg == nullptr)
                                               // 	{return {};}

        let mut detectorResult = Err(Exceptions::NOT_FOUND);
        if let Some(DecodeHintValue::PossibleFormats(formats)) =
            hints.get(&DecodeHintType::POSSIBLE_FORMATS)
        {
            if formats.contains(&BarcodeFormat::QR_CODE) {
                detectorResult = DetectPureQR(binImg);
            }
            if formats.contains(&BarcodeFormat::MICRO_QR_CODE) && detectorResult.is_err() {
                detectorResult = DetectPureMQR(binImg);
            }
        }

        if detectorResult.is_err() {
            for decode_function in [DetectPureQR, DetectPureMQR] {
                detectorResult = decode_function(binImg);
                if detectorResult.is_ok() {
                    break;
                }
            }
        }

        let detectorResult = detectorResult?;

        // let detectorResult: DetectorResult;
        // if (_hints.hasFormat(BarcodeFormat::QR_CODE))
        // 	{detectorResult = DetectPureQR(binImg);}

        // if (_hints.hasFormat(BarcodeFormat::MICRO_QR_CODE) && !detectorResult.isValid())
        // 	{detectorResult = DetectPureMQR(binImg);}

        // if (!detectorResult.isValid())
        // 	{return {};}

        let decoderResult = Decode(detectorResult.getBits())?;
        let position = detectorResult.getPoints();

        Ok(RXingResult::with_decoder_result(
            decoderResult,
            position,
            if detectorResult.getBits().width() < 21 {
                BarcodeFormat::MICRO_QR_CODE
            } else {
                BarcodeFormat::QR_CODE
            },
        ))

        // Ok(RXingResult::new(
        //     &decoderResult.content().to_string(),
        //     decoderResult.content().bytes().to_vec(),
        //     position.to_vec(),
        //     if detectorResult.getBits().width() < 21 {
        //         BarcodeFormat::MICRO_QR_CODE
        //     } else {
        //         BarcodeFormat::QR_CODE
        //     },
        // ))
        // return Result(std::move(decoderResult), std::move(position),
        // 			  detectorResult.bits().width() < 21 ? BarcodeFormat::MICRO_QR_CODE : BarcodeFormat::QR_CODE);
    }
}

impl MultipleBarcodeReader for QrReader {
    fn decode_multiple<B: crate::Binarizer>(
        &mut self,
        image: &mut crate::BinaryBitmap<B>,
    ) -> crate::common::Result<Vec<crate::RXingResult>> {
        self.decode_multiple_with_hints(image, &DecodingHintDictionary::new())
    }

    fn decode_multiple_with_hints<B: crate::Binarizer>(
        &mut self,
        image: &mut crate::BinaryBitmap<B>,
        hints: &DecodingHintDictionary,
    ) -> crate::common::Result<Vec<crate::RXingResult>> {
        self.decode_set_number_with_hints(image, hints, u32::MAX)
    }
}

impl QrReader {
    fn decode_set_number_with_hints<B: crate::Binarizer>(
        &mut self,
        image: &mut crate::BinaryBitmap<B>,
        hints: &DecodingHintDictionary,
        count: u32,
    ) -> crate::common::Result<Vec<RXingResult>> {
        let binImg = image.get_black_matrix(); //image.getBitMatrix();
        let maxSymbols = count;
        // if (binImg == nullptr)
        // 	{return {};}

        // #ifdef PRINT_DEBUG
        // 	LogMatrixWriter lmw(log, *binImg, 5, "qr-log.pnm");
        // #endif
        let try_harder = matches!(
            hints.get(&DecodeHintType::TRY_HARDER),
            Some(DecodeHintValue::TryHarder(true))
        );

        let mut allFPs = FindFinderPatterns(binImg, try_harder);

        // #ifdef PRINT_DEBUG
        // 	printf("allFPs: %d\n", Size(allFPs));
        // #endif

        let mut usedFPs: Vec<ConcentricPattern> = Vec::new();
        let mut results: Vec<RXingResult> = Vec::new();

        let (check_qr, check_mqr) = if let Some(DecodeHintValue::PossibleFormats(formats)) =
            hints.get(&DecodeHintType::POSSIBLE_FORMATS)
        {
            (
                formats.contains(&BarcodeFormat::QR_CODE),
                formats.contains(&BarcodeFormat::MICRO_QR_CODE),
            )
        } else {
            (true, true)
        };

        if check_qr {
            // if (_hints.hasFormat(BarcodeFormat::QRCode)) {
            let allFPSets = GenerateFinderPatternSets(&mut allFPs);
            for fpSet in allFPSets {
                // for (const auto& fpSet : allFPSets) {
                if (usedFPs.contains(&fpSet.bl)
                    || usedFPs.contains(&fpSet.tl)
                    || usedFPs.contains(&fpSet.tr))
                {
                    continue;
                }

                // logFPSet(fpSet);

                let detectorResult = SampleQR(binImg, &fpSet);
                if let Ok(detectorResult) = detectorResult {
                    // if (detectorResult.is_ok()) {
                    let decoderResult = Decode(detectorResult.getBits());
                    let position = detectorResult.getPoints();
                    if let Ok(decoderResult) = decoderResult {
                        if (decoderResult.isValid()) {
                            usedFPs.push(fpSet.bl);
                            usedFPs.push(fpSet.tl);
                            usedFPs.push(fpSet.tr);
                        }

                        if (decoderResult.isValid()) {
                            // results.push(RXingResult::new(
                            //     &decoderResult.content().to_string(),
                            //     decoderResult.content().bytes().to_vec(),
                            //     position.to_vec(),
                            //     BarcodeFormat::QR_CODE,
                            // ));
                            results.push(RXingResult::with_decoder_result(
                                decoderResult,
                                position,
                                BarcodeFormat::QR_CODE,
                            ));
                            // results.emplace_back(std::move(decoderResult), std::move(position), BarcodeFormat::QR_CODE);

                            if (maxSymbols != 0 && (results.len() as u32) == maxSymbols) {
                                break;
                            }
                        }
                    }
                }
            }
        }
        if (check_mqr && !(maxSymbols != 0 && (results.len() as u32) == maxSymbols)) {
            // if (_hints.hasFormat(BarcodeFormat::MicroQRCode) && !(maxSymbols && Size(results) == maxSymbols)) {
            for fp in allFPs {
                // for (const auto& fp : allFPs) {
                if (usedFPs.contains(&fp)) {
                    continue;
                }

                let detectorResult = SampleMQR(binImg, fp);
                if let Ok(detectorResult) = detectorResult {
                    // if (detectorResult.is_ok()) {
                    let decoderResult = Decode(detectorResult.getBits());
                    let position = detectorResult.getPoints();
                    if let Ok(decoderResult) = decoderResult {
                        if (decoderResult.isValid()) {
                            results.push(RXingResult::with_decoder_result(
                                decoderResult,
                                position,
                                BarcodeFormat::MICRO_QR_CODE,
                            ));
                            // results.push(RXingResult::new(
                            //     &decoderResult.content().to_string(),
                            //     decoderResult.content().bytes().to_vec(),
                            //     position.to_vec(),
                            //     BarcodeFormat::MICRO_QR_CODE,
                            // ));
                            // results.emplace_back(std::move(decoderResult), std::move(position), BarcodeFormat::MICRO_QR_CODE);

                            if (maxSymbols != 0 && (results.len() as u32) == maxSymbols) {
                                break;
                            }
                        }
                    }
                }
            }
        }

        return Ok(results);
    }
}
