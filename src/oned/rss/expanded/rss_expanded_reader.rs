/*
 * Copyright (C) 2010 ZXing authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/*
 * These authors would like to acknowledge the Spanish Ministry of Industry,
 * Tourism and Trade, for the support in the project TSI020301-2008-2
 * "PIRAmIDE: Personalizable Interactions with Resources on AmI-enabled
 * Mobile Dynamic Environments", led by Treelogic
 * ( http://www.treelogic.com/ ):
 *
 *   http://www.piramidepse.com/
 */

use std::collections::HashMap;

use crate::{
    common::{detector::MathUtils, BitArray},
    oned::{
        recordPattern, recordPatternInReverse,
        rss::{
            rss_utils, AbstractRSSReaderTrait, DataCharacter, DataCharacterTrait, FinderPattern,
            Pair,
        },
        OneDReader,
    },
    BarcodeFormat, DecodeHintType, DecodingHintDictionary, Exceptions, RXingResult,
    RXingResultMetadataType, RXingResultMetadataValue, RXingResultPoint, Reader, ResultPoint,
};

use super::{bit_array_builder, decoders::abstract_expanded_decoder, ExpandedPair, ExpandedRow};

const FINDER_PAT_A: u32 = 0;
const FINDER_PAT_B: u32 = 1;
const FINDER_PAT_C: u32 = 2;
const FINDER_PAT_D: u32 = 3;
const FINDER_PAT_E: u32 = 4;
const FINDER_PAT_F: u32 = 5;

use lazy_static::lazy_static;
lazy_static! {
    static ref FINDER_PATTERN_SEQUENCES: Vec<Vec<u32>> = vec![
        vec![FINDER_PAT_A, FINDER_PAT_A],
        vec![FINDER_PAT_A, FINDER_PAT_B, FINDER_PAT_B],
        vec![FINDER_PAT_A, FINDER_PAT_C, FINDER_PAT_B, FINDER_PAT_D],
        vec![
            FINDER_PAT_A,
            FINDER_PAT_E,
            FINDER_PAT_B,
            FINDER_PAT_D,
            FINDER_PAT_C
        ],
        vec![
            FINDER_PAT_A,
            FINDER_PAT_E,
            FINDER_PAT_B,
            FINDER_PAT_D,
            FINDER_PAT_D,
            FINDER_PAT_F
        ],
        vec![
            FINDER_PAT_A,
            FINDER_PAT_E,
            FINDER_PAT_B,
            FINDER_PAT_D,
            FINDER_PAT_E,
            FINDER_PAT_F,
            FINDER_PAT_F
        ],
        vec![
            FINDER_PAT_A,
            FINDER_PAT_A,
            FINDER_PAT_B,
            FINDER_PAT_B,
            FINDER_PAT_C,
            FINDER_PAT_C,
            FINDER_PAT_D,
            FINDER_PAT_D
        ],
        vec![
            FINDER_PAT_A,
            FINDER_PAT_A,
            FINDER_PAT_B,
            FINDER_PAT_B,
            FINDER_PAT_C,
            FINDER_PAT_C,
            FINDER_PAT_D,
            FINDER_PAT_E,
            FINDER_PAT_E
        ],
        vec![
            FINDER_PAT_A,
            FINDER_PAT_A,
            FINDER_PAT_B,
            FINDER_PAT_B,
            FINDER_PAT_C,
            FINDER_PAT_C,
            FINDER_PAT_D,
            FINDER_PAT_E,
            FINDER_PAT_F,
            FINDER_PAT_F
        ],
        vec![
            FINDER_PAT_A,
            FINDER_PAT_A,
            FINDER_PAT_B,
            FINDER_PAT_B,
            FINDER_PAT_C,
            FINDER_PAT_D,
            FINDER_PAT_D,
            FINDER_PAT_E,
            FINDER_PAT_E,
            FINDER_PAT_F,
            FINDER_PAT_F
        ],
    ];
}

/**
 * @author Pablo Orduña, University of Deusto (pablo.orduna@deusto.es)
 * @author Eduardo Castillejo, University of Deusto (eduardo.castillejo@deusto.es)
 */
pub struct RSSExpandedReader {
    _possibleLeftPairs: Vec<Pair>,
    _possibleRightPairs: Vec<Pair>,
    decodeFinderCounters: [u32; 4],
    dataCharacterCounters: [u32; 8],
    oddRoundingErrors: [f32; 4],
    evenRoundingErrors: [f32; 4],
    oddCounts: [u32; 4],
    evenCounts: [u32; 4],

    pub(super) pairs: Vec<ExpandedPair>, //new ArrayList<>(MAX_PAIRS);
    pub(super) rows: Vec<ExpandedRow>,   // new ArrayList<>();
    startEnd: [u32; 2],                  // new int[2];
    startFromEven: bool,
}
impl AbstractRSSReaderTrait for RSSExpandedReader {}
impl OneDReader for RSSExpandedReader {
    fn decodeRow(
        &mut self,
        rowNumber: u32,
        row: &crate::common::BitArray,
        _hints: &crate::DecodingHintDictionary,
    ) -> Result<crate::RXingResult, crate::Exceptions> {
        // Rows can start with even pattern in case in prev rows there where odd number of patters.
        // So lets try twice
        self.pairs.clear();
        self.startFromEven = false;
        // try {
            if let Ok(decoded_two_pairs) = self.decodeRow2pairs(rowNumber, row) {
                if let Ok(possible_result) = Self::constructRXingResult(&decoded_two_pairs) {
                    return Ok(possible_result)
                }
            }
        // let possible = Self::constructRXingResult(&self.decodeRow2pairs(rowNumber, row)?);
        // if possible.is_ok() {
        //     return possible;
        // }
        // } catch (NotFoundException e) {
        // OK
        // }

        self.pairs.clear();
        self.startFromEven = true;
        Self::constructRXingResult(&self.decodeRow2pairs(rowNumber, row)?)
    }
}
impl Reader for RSSExpandedReader {
    fn decode(&mut self, image: &crate::BinaryBitmap) -> Result<crate::RXingResult, Exceptions> {
        self.decode_with_hints(image, &HashMap::new())
    }

    // Note that we don't try rotation without the try harder flag, even if rotation was supported.
    fn decode_with_hints(
        &mut self,
        image: &crate::BinaryBitmap,
        hints: &DecodingHintDictionary,
    ) -> Result<crate::RXingResult, Exceptions> {
        if let Ok(res) = self.doDecode(image, hints) {
            Ok(res)
        } else {
            let tryHarder = hints.contains_key(&DecodeHintType::TRY_HARDER);
            if tryHarder && image.isRotateSupported() {
                let rotatedImage = image.rotateCounterClockwise();
                let mut result = self.doDecode(&rotatedImage, hints)?;
                // Record that we found it rotated 90 degrees CCW / 270 degrees CW
                let metadata = result.getRXingResultMetadata();
                let mut orientation = 270;
                if metadata.contains_key(&RXingResultMetadataType::ORIENTATION) {
                    // But if we found it reversed in doDecode(), add in that result here:
                    orientation = (orientation
                        + if let Some(crate::RXingResultMetadataValue::Orientation(or)) =
                            metadata.get(&RXingResultMetadataType::ORIENTATION)
                        {
                            *or
                        } else {
                            0
                        })
                        % 360;
                }
                result.putMetadata(
                    RXingResultMetadataType::ORIENTATION,
                    RXingResultMetadataValue::Orientation(orientation),
                );
                // Update result points
                // let points = result.getRXingResultPoints();
                // if points != null {
                let height = rotatedImage.getHeight();
                // for point in result.getRXingResultPointsMut().iter_mut() {
                let total_points = result.getRXingResultPoints().len();
                let points = result.getRXingResultPointsMut();
                for i in 0..total_points {
                    // for (int i = 0; i < points.length; i++) {
                    points[i] = RXingResultPoint::new(
                        height as f32 - points[i].getY() - 1.0,
                        points[i].getX(),
                    );
                }
                // }

                Ok(result)
            } else {
                return Err(Exceptions::NotFoundException("".to_owned()));
            }
        }
    }

    fn reset(&mut self) {
        self.pairs.clear();
        self.rows.clear();
    }
}

impl RSSExpandedReader {
    pub fn new() -> Self {
        Self::default()
    }
    const SYMBOL_WIDEST: [u32; 5] = [7, 5, 4, 3, 1];
    const EVEN_TOTAL_SUBSET: [u32; 5] = [4, 20, 52, 104, 204];
    const GSUM: [u32; 5] = [0, 348, 1388, 2948, 3988];

    const FINDER_PATTERNS: [[u32; 4]; 6] = [
        [1, 8, 4, 1], // A
        [3, 6, 4, 1], // B
        [3, 4, 6, 1], // C
        [3, 2, 8, 1], // D
        [2, 6, 5, 1], // E
        [2, 2, 9, 1], // F
    ];

    const WEIGHTS: [[u32; 8]; 23] = [
        [1, 3, 9, 27, 81, 32, 96, 77],
        [20, 60, 180, 118, 143, 7, 21, 63],
        [189, 145, 13, 39, 117, 140, 209, 205],
        [193, 157, 49, 147, 19, 57, 171, 91],
        [62, 186, 136, 197, 169, 85, 44, 132],
        [185, 133, 188, 142, 4, 12, 36, 108],
        [113, 128, 173, 97, 80, 29, 87, 50],
        [150, 28, 84, 41, 123, 158, 52, 156],
        [46, 138, 203, 187, 139, 206, 196, 166],
        [76, 17, 51, 153, 37, 111, 122, 155],
        [43, 129, 176, 106, 107, 110, 119, 146],
        [16, 48, 144, 10, 30, 90, 59, 177],
        [109, 116, 137, 200, 178, 112, 125, 164],
        [70, 210, 208, 202, 184, 130, 179, 115],
        [134, 191, 151, 31, 93, 68, 204, 190],
        [148, 22, 66, 198, 172, 94, 71, 2],
        [6, 18, 54, 162, 64, 192, 154, 40],
        [120, 149, 25, 75, 14, 42, 126, 167],
        [79, 26, 78, 23, 69, 207, 199, 175],
        [103, 98, 83, 38, 114, 131, 182, 124],
        [161, 61, 183, 127, 170, 88, 53, 159],
        [55, 165, 73, 8, 24, 72, 5, 15],
        [45, 135, 194, 160, 58, 174, 100, 89],
    ];

    const MAX_PAIRS: usize = 11;

    // Not private for testing
    pub(super) fn decodeRow2pairs(
        &mut self,
        rowNumber: u32,
        row: &BitArray,
    ) -> Result<Vec<ExpandedPair>, Exceptions> {
        let mut done = false;
        while !done {
            let previousPairs = self.pairs.clone();
            let to_add_res = self.retrieveNextPair(row, &previousPairs, rowNumber);
            if let Ok(to_add) = to_add_res {
                self.pairs.push(to_add);
            } else if self.pairs.is_empty() {
                return Err(to_add_res.err().unwrap());
            } else {
                // exit this loop when retrieveNextPair() fails and throws
                done = true;
            }
            // self.pairs = previousPairs;
        }
        //   try {
        //     this.pairs.add(retrieveNextPair(row, this.pairs, rowNumber));
        //   } catch (NotFoundException nfe) {
        //     if (this.pairs.isEmpty()) {
        //       throw nfe;
        //     }
        //     // exit this loop when retrieveNextPair() fails and throws
        //     done = true;
        //   }
        // }

        // TODO: verify sequence of finder patterns as in checkPairSequence()
        if self.checkChecksum() {
            return Ok(self.pairs.clone());
        }

        let tryStackedDecode = !self.rows.is_empty();
        self.storeRow(rowNumber); // TODO: deal with reversed rows
        if tryStackedDecode {
            // When the image is 180-rotated, then rows are sorted in wrong direction.
            // Try twice with both the directions.
            let ps = self.checkRows(false);
            if ps.is_some() {
                return Ok(ps.unwrap());
            }
            let ps = self.checkRows(true);
            if ps.is_some() {
                return Ok(ps.unwrap());
            }
        }

        return Err(Exceptions::NotFoundException("".to_owned()));
    }

    fn checkRows(&mut self, reverse: bool) -> Option<Vec<ExpandedPair>> {
        // Limit number of rows we are checking
        // We use recursive algorithm with pure complexity and don't want it to take forever
        // Stacked barcode can have up to 11 rows, so 25 seems reasonable enough
        if self.rows.len() > 25 {
            self.rows.clear(); // We will never have a chance to get result, so clear it
            return None;
        }

        self.pairs.clear();
        if reverse {
            self.rows.reverse();
        }

        let mut c_rows = Vec::new();
        let ps = if let Ok(res) = self.checkRowsDetails(&mut c_rows, 0) {
            Some(res)
        } else {
            None
        };
        // } catch (NotFoundException e) {
        // OK
        // }

        if reverse {
            self.rows.reverse();
        }

        ps
    }

    // Try to construct a valid rows sequence
    // Recursion is used to implement backtracking
    fn checkRowsDetails(
        &mut self,
        collectedRows: &mut Vec<ExpandedRow>,
        currentRow: usize,
    ) -> Result<Vec<ExpandedPair>, Exceptions> {
        for i in currentRow..self.rows.len() {
            // for (int i = currentRow; i < rows.size(); i++) {
            let row = self.rows.get(i).unwrap();
            self.pairs.clear();
            for collectedRow in &collectedRows.clone() {
                // for (ExpandedRow collectedRow : collectedRows) {

                self.pairs.append(&mut collectedRow.getPairs().to_vec());
            }
            self.pairs.append(&mut row.getPairs().to_vec());

            if Self::isValidSequence(&self.pairs) {
                if self.checkChecksum() {
                    return Ok(self.pairs.clone());
                }

                // let rs =  collectedRows;
                collectedRows.push(row.clone());
                // try {
                // Recursion: try to add more rows
                if let Ok(cr) = self.checkRowsDetails(collectedRows, i + 1) {
                    return Ok(cr);
                }
                // return checkRows(rs, i + 1);
                // } catch (NotFoundException e) {
                // We failed, try the next candidate
                // }
            }
        }

        return Err(Exceptions::NotFoundException("".to_owned()));
    }

    // Whether the pairs form a valid find pattern sequence,
    // either complete or a prefix
    fn isValidSequence(pairs: &[ExpandedPair]) -> bool {
        for i in 0..FINDER_PATTERN_SEQUENCES.len() {
            // for sequence in &FINDER_PATTERN_SEQUENCES.iter() {
            let sequence = FINDER_PATTERN_SEQUENCES.get(i).unwrap();
            // for (int[] sequence : FINDER_PATTERN_SEQUENCES) {
            if pairs.len() <= sequence.len() {
                let mut stop = true;
                for j in 0..pairs.len() {
                    // for (int j = 0; j < pairs.size(); j++) {
                    if pairs
                        .get(j)
                        .unwrap()
                        .getFinderPattern()
                        .as_ref()
                        .unwrap()
                        .getValue()
                        != sequence[j]
                    {
                        stop = false;
                        break;
                    }
                }
                if stop {
                    return true;
                }
            }
        }

        return false;
    }

    fn storeRow(&mut self, rowNumber: u32) {
        // Discard if duplicate above or below; otherwise insert in order by row number.
        let mut insertPos = 0;
        let mut prevIsSame = false;
        let mut nextIsSame = false;
        while insertPos < self.rows.len() {
            let erow = self.rows.get(insertPos).unwrap();
            if erow.getRowNumber() > rowNumber {
                nextIsSame = erow.isEquivalent(&self.pairs);
                break;
            }
            prevIsSame = erow.isEquivalent(&self.pairs);
            insertPos += 1;
        }
        if nextIsSame || prevIsSame {
            return;
        }

        // When the row was partially decoded (e.g. 2 pairs found instead of 3),
        // it will prevent us from detecting the barcode.
        // Try to merge partial rows

        // Check whether the row is part of an already detected row
        if Self::isPartialRow(&self.pairs, &self.rows) {
            return;
        }

        self.rows
            .insert(insertPos, ExpandedRow::new(self.pairs.clone(), rowNumber));

        Self::removePartialRows(&self.pairs, &mut self.rows);
    }

    // Remove all the rows that contains only specified pairs
    fn removePartialRows(pairs: &[ExpandedPair], rows: &mut Vec<ExpandedRow>) {
        let row_search = rows.clone();
        for i in 0..row_search.len() {
            // for r in rows {
            // for (Iterator<ExpandedRow> iterator = rows.iterator(); iterator.hasNext();) {
            //   ExpandedRow r = iterator.next();
            let r = row_search.get(i).unwrap();
            if r.getPairs().len() != pairs.len() {
                let mut allFound = true;
                for p in r.getPairs() {
                    // for (ExpandedPair p : r.getPairs()) {
                    if !pairs.contains(p) {
                        allFound = false;
                        break;
                    }
                }
                if allFound {
                    // 'pairs' contains all the pairs from the row 'r'
                    // iterator.remove();
                    rows.remove(i);
                }
            }
        }
    }

    // Returns true when one of the rows already contains all the pairs
    fn isPartialRow(pairs: &[ExpandedPair], rows: &[ExpandedRow]) -> bool {
        for r in rows {
            // for (ExpandedRow r : rows) {
            let mut allFound = true;
            for p in pairs {
                // for (ExpandedPair p : pairs) {
                let mut found = false;
                for pp in r.getPairs() {
                    // for (ExpandedPair pp : r.getPairs()) {
                    if p == pp {
                        found = true;
                        break;
                    }
                }
                if !found {
                    allFound = false;
                    break;
                }
            }
            if allFound {
                // the row 'r' contain all the pairs from 'pairs'
                return true;
            }
        }
        return false;
    }

    // Only used for unit testing
    #[cfg(test)]
    pub(crate) fn getRowsMut(&mut self) -> &mut [ExpandedRow] {
        &mut self.rows
    }
    #[cfg(test)]
    pub(crate) fn getRows(&self) -> &[ExpandedRow] {
        &self.rows
    }

    // Not private for unit testing
    pub(crate) fn constructRXingResult(pairs: &[ExpandedPair]) -> Result<RXingResult, Exceptions> {
        let binary = bit_array_builder::buildBitArray(&pairs.to_vec());

        let mut decoder = abstract_expanded_decoder::createDecoder(&binary)?;
        let resultingString = decoder.parseInformation()?;

        let firstPoints = pairs
            .get(0)
            .unwrap()
            .getFinderPattern()
            .as_ref()
            .unwrap()
            .getRXingResultPoints();
        let lastPoints = pairs
            .get(pairs.len() - 1)
            .unwrap()
            .getFinderPattern()
            .as_ref()
            .unwrap()
            .getRXingResultPoints();

        let mut result = RXingResult::new(
            &resultingString,
            Vec::new(),
            vec![firstPoints[0], firstPoints[1], lastPoints[0], lastPoints[1]],
            BarcodeFormat::RSS_EXPANDED,
        );

        result.putMetadata(
            RXingResultMetadataType::SYMBOLOGY_IDENTIFIER,
            RXingResultMetadataValue::SymbologyIdentifier("]e0".to_owned()),
        );

        Ok(result)
    }

    fn checkChecksum(&self) -> bool {
        let firstPair = self.pairs.get(0).unwrap();
        let checkCharacter = firstPair.getLeftChar();
        let Some(firstCharacter) = firstPair.getRightChar() else {
      return false;
    };

        let mut checksum = firstCharacter.getChecksumPortion();
        let mut s = 2;

        // for i in 1..self.pairs.len() {
        // for (int i = 1; i < this.pairs.size(); ++i) {
        for currentPair in self.pairs.iter().skip(1) {
            // let currentPair = self.pairs.get(i).unwrap();
            checksum += currentPair.getLeftChar().unwrap().getChecksumPortion();
            s += 1;
            if let Some(currentRightChar) = currentPair.getRightChar() {
                // if (currentRightChar != null) {
                checksum += currentRightChar.getChecksumPortion();
                s += 1;
            }
        }

        checksum %= 211;

        let checkCharacterValue = (211 * (s as i64 - 4) + checksum as i64) as u32;

        checkCharacterValue == checkCharacter.unwrap().getValue()
    }

    fn getNextSecondBar(row: &BitArray, initialPos: usize) -> usize {
        let mut currentPos;
        if row.get(initialPos) {
            currentPos = row.getNextUnset(initialPos);
            currentPos = row.getNextSet(currentPos);
        } else {
            currentPos = row.getNextSet(initialPos);
            currentPos = row.getNextUnset(currentPos);
        }
        currentPos
    }

    // not private for testing
    pub(super) fn retrieveNextPair(
        &mut self,
        row: &BitArray,
        previousPairs: &[ExpandedPair],
        rowNumber: u32,
    ) -> Result<ExpandedPair, Exceptions> {
        let mut isOddPattern = previousPairs.len() % 2 == 0;
        if self.startFromEven {
            isOddPattern = !isOddPattern;
        }

        let mut pattern;

        let mut keepFinding = true;
        let mut forcedOffset = -1_i32;
        loop {
            self.findNextPair(row, previousPairs, forcedOffset)?;
            pattern = self.parseFoundFinderPattern(row, rowNumber, isOddPattern);
            if pattern.is_none() {
                forcedOffset = Self::getNextSecondBar(row, self.startEnd[0] as usize) as i32;
            } else {
                keepFinding = false;
            }
            if !keepFinding {
                break;
            }
        } //while (keepFinding);

        // When stacked symbol is split over multiple rows, there's no way to guess if this pair can be last or not.
        // boolean mayBeLast = checkPairSequence(previousPairs, pattern);

        let leftChar =
            self.decodeDataCharacter(row, pattern.as_ref().unwrap(), isOddPattern, true)?;

        if !previousPairs.is_empty()
            && previousPairs
                .get(previousPairs.len() - 1)
                .unwrap()
                .mustBeLast()
        {
            return Err(Exceptions::NotFoundException("".to_owned()));
        }

        let rightChar = if let Ok(ch) =
            self.decodeDataCharacter(row, pattern.as_ref().unwrap(), isOddPattern, false)
        {
            Some(ch)
        } else {
            None
        };
        // try {
        //   rightChar = this.decodeDataCharacter(row, pattern, isOddPattern, false);
        // } catch (NotFoundException ignored) {
        //   rightChar = null;
        // }

        Ok(ExpandedPair::new(Some(leftChar), rightChar, pattern))
    }

    fn findNextPair(
        &mut self,
        row: &BitArray,
        previousPairs: &[ExpandedPair],
        forcedOffset: i32,
    ) -> Result<(), Exceptions> {
        let counters = &mut self.decodeFinderCounters;
        counters[0] = 0;
        counters[1] = 0;
        counters[2] = 0;
        counters[3] = 0;

        let width = row.getSize();

        let mut rowOffset;
        if forcedOffset >= 0 {
            rowOffset = forcedOffset;
        } else if previousPairs.is_empty() {
            rowOffset = 0;
        } else {
            let lastPair = previousPairs.get(previousPairs.len() - 1).unwrap();
            rowOffset = lastPair.getFinderPattern().as_ref().unwrap().getStartEnd()[1] as i32;
        }
        let mut searchingEvenPair = previousPairs.len() % 2 != 0;
        if self.startFromEven {
            searchingEvenPair = !searchingEvenPair;
        }

        let mut isWhite = false;
        while rowOffset < width as i32 {
            isWhite = !row.get(rowOffset as usize);
            if !isWhite {
                break;
            }
            rowOffset += 1;
        }

        let mut counterPosition = 0;
        let mut patternStart = rowOffset;
        for x in rowOffset..width as i32 {
            // for (int x = rowOffset; x < width; x++) {
            if row.get(x as usize) != isWhite {
                counters[counterPosition] += 1;
            } else {
                if counterPosition == 3 {
                    if searchingEvenPair {
                        Self::reverseCounters(counters);
                    }

                    if Self::isFinderPattern(counters) {
                        self.startEnd[0] = patternStart as u32;
                        self.startEnd[1] = x as u32;
                        return Ok(());
                    }

                    if searchingEvenPair {
                        Self::reverseCounters(counters);
                    }

                    patternStart += (counters[0] + counters[1]) as i32;
                    counters[0] = counters[2];
                    counters[1] = counters[3];
                    counters[2] = 0;
                    counters[3] = 0;
                    counterPosition -= 1;
                } else {
                    counterPosition += 1;
                }
                counters[counterPosition] = 1;
                isWhite = !isWhite;
            }
        }
        return Err(Exceptions::NotFoundException("".to_owned()));
    }

    fn reverseCounters(counters: &mut [u32]) {
        let length = counters.len();
        for i in 0..length / 2 {
            // for (int i = 0; i < length / 2; ++i) {
            let tmp = counters[i];
            counters[i] = counters[length - i - 1];
            counters[length - i - 1] = tmp;
        }
    }

    fn parseFoundFinderPattern(
        &self,
        row: &BitArray,
        rowNumber: u32,
        oddPattern: bool,
    ) -> Option<FinderPattern> {
        // Actually we found elements 2-5.
        let firstCounter;
        let start;
        let end;

        if oddPattern {
            // If pattern number is odd, we need to locate element 1 *before* the current block.

            let mut firstElementStart = self.startEnd[0] as i32 - 1;
            // Locate element 1
            while firstElementStart >= 0 && !row.get(firstElementStart as usize) {
                firstElementStart -= 1;
            }

            firstElementStart += 1;
            firstCounter = self.startEnd[0] as i32 - firstElementStart;
            start = firstElementStart;
            end = self.startEnd[1];
        } else {
            // If pattern number is even, the pattern is reversed, so we need to locate element 1 *after* the current block.

            start = self.startEnd[0] as i32;

            end = row.getNextUnset(self.startEnd[1] as usize + 1) as u32;
            firstCounter = end as i32 - self.startEnd[1] as i32;
        }

        // Make 'counters' hold 1-4
        let mut counters = self.decodeFinderCounters;
        let slc = counters[..counters.len() - 1].to_vec();
        counters[1..].copy_from_slice(&slc);
        // System.arraycopy(counters, 0, counters, 1, counters.length - 1);

        counters[0] = firstCounter as u32;
        let Ok(value) = Self::parseFinderValue(&counters, &Self::FINDER_PATTERNS) else {
      return None
    };

        Some(FinderPattern::new(
            value,
            [start as usize, end as usize],
            start as usize,
            end as usize,
            rowNumber,
        ))
    }

    pub(super) fn decodeDataCharacter(
        &mut self,
        row: &BitArray,
        pattern: &FinderPattern,
        isOddPattern: bool,
        leftChar: bool,
    ) -> Result<DataCharacter, Exceptions> {
        let counters = &mut self.dataCharacterCounters;
        counters.fill(0);

        if leftChar {
            recordPatternInReverse(row, pattern.getStartEnd()[0], counters)?;
        } else {
            recordPattern(row, pattern.getStartEnd()[1], counters)?;
            // reverse it
            let mut i = 0;
            let mut j = counters.len() - 1;
            while i < j {
                // for (int i = 0, j = counters.length - 1; i < j; i++, j--) {
                let temp = counters[i];
                counters[i] = counters[j];
                counters[j] = temp;

                i += 1;
                j -= 1;
            }
        } //counters[] has the pixels of the module

        let numModules = 17; //left and right data characters have all the same length
                             // let elementWidth: f32 = MathUtils::sum(counters) / numModules;
        let elementWidth: f32 = (counters.iter().sum::<u32>() as f32) / numModules as f32;

        // Sanity check: element width for pattern and the character should match
        let expectedElementWidth: f32 =
            (pattern.getStartEnd()[1] - pattern.getStartEnd()[0]) as f32 / 15.0;
        if (elementWidth - expectedElementWidth).abs() / expectedElementWidth > 0.3 {
            return Err(Exceptions::NotFoundException("".to_owned()));
        }

        // let oddCounts = &mut self.oddCounts;
        // let evenCounts = &mut self.evenCounts;
        // let oddRoundingErrors = &mut self.oddRoundingErrors;
        // let evenRoundingErrors = &mut self.evenRoundingErrors;

        for i in 0..counters.len() {
            // for (int i = 0; i < counters.length; i++) {
            let value: f32 = 1.0 * counters[i] as f32 / elementWidth;
            let mut count = (value + 0.5) as i32; // Round
            if count < 1 {
                if value < 0.3 {
                    return Err(Exceptions::NotFoundException("".to_owned()));
                }
                count = 1;
            } else if count > 8 {
                if value > 8.7 {
                    return Err(Exceptions::NotFoundException("".to_owned()));
                }
                count = 8;
            }
            let offset = i / 2;
            if (i & 0x01) == 0 {
                self.oddCounts[offset] = count as u32;
                self.oddRoundingErrors[offset] = value - count as f32;
            } else {
                self.evenCounts[offset] = count as u32;
                self.evenRoundingErrors[offset] = value - count as f32;
            }
        }

        self.adjustOddEvenCounts(numModules as u32)?;

        let weightRowNumber = (4 * pattern.getValue() as isize
            + (if isOddPattern { 0 } else { 2 })
            + (if leftChar { 0 } else { 1 })
            - 1) as usize;

        let mut oddSum = 0;
        let mut oddChecksumPortion = 0;
        for i in (0..self.oddCounts.len()).rev() {
            // for (int i = oddCounts.length - 1; i >= 0; i--) {
            if Self::isNotA1left(pattern, isOddPattern, leftChar) {
                let weight = Self::WEIGHTS[weightRowNumber][2 * i];
                oddChecksumPortion += self.oddCounts[i] * weight;
            }
            oddSum += self.oddCounts[i];
        }
        let mut evenChecksumPortion = 0;
        for i in (0..self.evenCounts.len()).rev() {
            //  for (int i = evenCounts.length - 1; i >= 0; i--) {
            if Self::isNotA1left(pattern, isOddPattern, leftChar) {
                let weight = Self::WEIGHTS[weightRowNumber][2 * i + 1];
                evenChecksumPortion += self.evenCounts[i] * weight;
            }
        }
        let checksumPortion = oddChecksumPortion + evenChecksumPortion;

        if (oddSum & 0x01) != 0 || oddSum > 13 || oddSum < 4 {
            return Err(Exceptions::NotFoundException("".to_owned()));
        }

        let group = ((13 - oddSum) / 2) as usize;
        let oddWidest = Self::SYMBOL_WIDEST[group];
        let evenWidest = 9 - oddWidest;
        let vOdd = rss_utils::getRSSvalue(&self.oddCounts, oddWidest, true);
        let vEven = rss_utils::getRSSvalue(&self.evenCounts, evenWidest, false);
        let tEven = Self::EVEN_TOTAL_SUBSET[group];
        let gSum = Self::GSUM[group];
        let value = vOdd * tEven + vEven + gSum;

        Ok(DataCharacter::new(value, checksumPortion))
    }

    fn isNotA1left(pattern: &FinderPattern, isOddPattern: bool, leftChar: bool) -> bool {
        // A1: pattern.getValue is 0 (A), and it's an oddPattern, and it is a left char
        !(pattern.getValue() == 0 && isOddPattern && leftChar)
    }

    fn adjustOddEvenCounts(&mut self, numModules: u32) -> Result<(), Exceptions> {
        // let oddSum = MathUtils::sum(&self.oddCounts);
        // let evenSum = MathUtils::sum(&self.evenCounts);
        let oddSum = self.oddCounts.iter().sum::<u32>();
        let evenSum = self.evenCounts.iter().sum::<u32>();

        let mut incrementOdd = false;
        let mut decrementOdd = false;

        if oddSum > 13 {
            decrementOdd = true;
        } else if oddSum < 4 {
            incrementOdd = true;
        }
        let mut incrementEven = false;
        let mut decrementEven = false;
        if evenSum > 13 {
            decrementEven = true;
        } else if evenSum < 4 {
            incrementEven = true;
        }

        let mismatch = oddSum as isize + evenSum as isize - numModules as isize;
        let oddParityBad = (oddSum & 0x01) == 1;
        let evenParityBad = (evenSum & 0x01) == 0;
        match mismatch {
            1 => {
                if oddParityBad {
                    if evenParityBad {
                        return Err(Exceptions::NotFoundException("".to_owned()));
                    }
                    decrementOdd = true;
                } else {
                    if !evenParityBad {
                        return Err(Exceptions::NotFoundException("".to_owned()));
                    }
                    decrementEven = true;
                }
            }
            -1 => {
                if oddParityBad {
                    if evenParityBad {
                        return Err(Exceptions::NotFoundException("".to_owned()));
                    }
                    incrementOdd = true;
                } else {
                    if !evenParityBad {
                        return Err(Exceptions::NotFoundException("".to_owned()));
                    }
                    incrementEven = true;
                }
            }
            0 => {
                if oddParityBad {
                    if !evenParityBad {
                        return Err(Exceptions::NotFoundException("".to_owned()));
                    }
                    // Both bad
                    if oddSum < evenSum {
                        incrementOdd = true;
                        decrementEven = true;
                    } else {
                        decrementOdd = true;
                        incrementEven = true;
                    }
                } else {
                    if evenParityBad {
                        return Err(Exceptions::NotFoundException("".to_owned()));
                    }
                    // Nothing to do!
                }
            }

            _ => return Err(Exceptions::NotFoundException("".to_owned())),
        }

        if incrementOdd {
            if decrementOdd {
                return Err(Exceptions::NotFoundException("".to_owned()));
            }
            Self::increment(&mut self.oddCounts, &self.oddRoundingErrors);
        }
        if decrementOdd {
            Self::decrement(&mut self.oddCounts, &self.oddRoundingErrors);
        }
        if incrementEven {
            if decrementEven {
                return Err(Exceptions::NotFoundException("".to_owned()));
            }
            Self::increment(&mut self.evenCounts, &self.oddRoundingErrors);
        }
        if decrementEven {
            Self::decrement(&mut self.evenCounts, &self.evenRoundingErrors);
        }

        Ok(())
    }
}

impl Default for RSSExpandedReader {
    fn default() -> Self {
        Self {
            _possibleLeftPairs: Default::default(),
            _possibleRightPairs: Default::default(),
            decodeFinderCounters: Default::default(),
            dataCharacterCounters: Default::default(),
            oddRoundingErrors: Default::default(),
            evenRoundingErrors: Default::default(),
            oddCounts: Default::default(),
            evenCounts: Default::default(),
            pairs: Default::default(),
            rows: Default::default(),
            startEnd: Default::default(),
            startFromEven: Default::default(),
        }
    }
}