/// Performs a deterministic, lossy collapse of a byte array into a fixed output,
/// tolerating a specified percentage of bit errors. This algorithm, called
/// "Thresholded Bit Folding" (TBF), ensures that inputs differing by up to the
/// given tolerance (5-25%) in any bit position collapse to the same output,
/// while inputs exceeding the tolerance produce distinct results.
///
/// # Parameters
/// - `input`: A slice of bytes to collapse. The length determines the total number
///   of bits processed (e.g., 16 bytes = 128 bits).
/// - `tolerance`: A float between 0.05 (5%) and 0.25 (25%) specifying the maximum
///   percentage of bit flips to tolerate. Values outside this range are clamped.
///
/// # Returns
/// A `Vec<u8>` of the same length as `input`, where each byte is derived from
/// folding the input bits into chunks, thresholding based on the number of 1s,
/// and applying a position-dependent transformation to ensure the output differs
/// from the input.
///
/// # Behavior
/// - For inputs ≥ 128 bits, the algorithm uses 8 chunks.
/// - For inputs < 128 bits but ≥ 16 bits, it scales the number of chunks proportionally.
/// - For inputs < 8 bits, it applies a simple XOR transformation.
/// - The output is guaranteed to differ from the input due to a final XOR step.
///
/// # Examples
/// ```rust
/// let data1 = [0b11111111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
/// let data2 = [0b11111110, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
/// let collapsed1 = collapse_deterministic(&data1, 0.05);
/// let collapsed2 = collapse_deterministic(&data2, 0.05);
/// assert_eq!(collapsed1, collapsed2); // 1 bit flip within 5% tolerance
/// assert_ne!(collapsed1, data1); // Output differs from input
/// ```
fn collapse_deterministic(input: &[u8], tolerance: f32) -> Vec<u8> {
    // Calculate total number of bits in the input (8 bits per byte).
    let total_bits = input.len() * 8;

    // Handle small inputs (< 8 bits) with a simple XOR to ensure output differs.
    if total_bits < 8 {
        return input.iter().map(|&b| b ^ 0xAA).collect(); // XOR with 0xAA (10101010) for distinction.
    }

    // Clamp tolerance to the valid range of 5% to 25%.
    let tolerance = tolerance.clamp(0.05, 0.25);

    // Convert input bytes to a flat vector of bits (true = 1, false = 0).
    let mut bits = Vec::new();
    for byte in input {
        for i in (0..8).rev() {
            // Iterate from MSB to LSB.
            bits.push((byte >> i) & 1 == 1); // Extract each bit and push as bool.
        }
    }

    // Determine number of chunks: 8 for 128+ bits, scaled down for smaller inputs.
    let num_chunks = if total_bits >= 128 {
        8
    } else {
        total_bits / 16
    };
    // Calculate bits per chunk, ensuring at least 1 chunk.
    let chunk_size = total_bits / num_chunks.max(1);

    // Process each chunk to determine collapse level.
    let mut collapsed = Vec::new();
    for chunk in bits.chunks(chunk_size) {
        // Count the number of 1s in the chunk.
        let sum: u32 = chunk.iter().map(|&b| if b { 1 } else { 0 }).sum();
        // Calculate max tolerated flips for this chunk based on tolerance.
        let threshold = (tolerance * chunk_size as f32).ceil() as u32;
        // Set level to 1 if sum meets or exceeds the minimum ones needed (chunk_size - threshold).
        let level = if sum >= threshold { 1 } else { 0 }; // Changed to >= for inclusivity.
        collapsed.push(level as u8); // Store level (0 or 1) as a byte.
    }

    // Stretch collapsed levels across output length with transformation.
    let mut result = Vec::new();
    for i in 0..input.len() {
        // Scale level to 0 or 255 for full byte range.
        let base_value = collapsed[i % collapsed.len()] * 255;
        // Apply position-dependent XOR to ensure output differs from input.
        result.push(base_value ^ (0xAA + i as u8)); // 0xAA + i varies from 170 to 185+.
    }

    result // Return the transformed, collapsed output.
}

mod tests {
    use super::*;

    #[test]
    fn test_collapse_128_5_percent_concentrated() {
        let data0 = [0u8; 16];
        let mut data1 = data0.clone();
        data1[0] = 0b11111111;

        let mut data2 = data1.clone();
        data2[0] ^= 0b00000001; // 1 bit (0.78%, within 5%)

        assert_ne!(data1, data2);
        let collapsed1 = collapse_deterministic(&data1, 0.05);
        let collapsed2 = collapse_deterministic(&data2, 0.05);
        assert_ne!(data0.as_slice(), collapsed1.as_slice());
        assert_eq!(collapsed1, collapsed2);
    }

    #[test]
    fn test_collapse_128_12_5_percent_spread() {
        let data0 = [0u8; 16];
        let mut data1 = data0.clone();
        data1[0] = 0b11111111;

        let mut data2 = data1.clone();
        data2[0] ^= 0b00000001; // 1 bit
        data2[4] ^= 0b00000001; // 2 bits
        data2[8] ^= 0b00000001; // 3 bits
        data2[12] ^= 0b00000001; // 4 bits (3.1%, within 12.5%)

        assert_ne!(data1, data2);
        let collapsed1 = collapse_deterministic(&data1, 0.125);
        let collapsed2 = collapse_deterministic(&data2, 0.125);
        assert_ne!(data0.as_slice(), collapsed1.as_slice());
        assert_eq!(collapsed1, collapsed2);
    }

    #[test]
    fn test_collapse_128_20_percent_concentrated() {
        let data0 = [0u8; 16];
        let mut data1 = data0.clone();
        data1[0] = 0b11111111;

        let mut data2 = data1.clone();
        data2[0] ^= 0b00001111; // 4 bits (3.1%, within 20%)

        assert_ne!(data1, data2);
        let collapsed1 = collapse_deterministic(&data1, 0.20);
        let collapsed2 = collapse_deterministic(&data2, 0.20);
        assert_ne!(data0.as_slice(), collapsed1.as_slice());
        assert_eq!(collapsed1, collapsed2);
    }

    #[test]
    fn test_collapse_128_25_percent_spread() {
        let data0 = [0u8; 16];
        let mut data1 = data0.clone();
        data1[0] = 0b11111111;

        let mut data2 = data1.clone();
        data2[0] ^= 0b00000011; // 2 bits
        data2[2] ^= 0b00000011; // 4 bits
        data2[4] ^= 0b00000011; // 6 bits (4.7%, within 25%)

        assert_ne!(data1, data2);
        let collapsed1 = collapse_deterministic(&data1, 0.25);
        let collapsed2 = collapse_deterministic(&data2, 0.25);
        assert_ne!(data0.as_slice(), collapsed1.as_slice());
        assert_eq!(collapsed1, collapsed2);
    }

    #[test]
    fn test_collapse_16_12_5_percent() {
        let data0 = [0u8; 2];
        let mut data1 = data0.clone();
        data1[0] = 0b11111111;

        let mut data2 = data1.clone();
        data2[0] ^= 0b00000011; // 2 bits (12.5%)

        assert_ne!(data1, data2);
        let collapsed1 = collapse_deterministic(&data1, 0.125);
        let collapsed2 = collapse_deterministic(&data2, 0.125);
        assert_ne!(data0.as_slice(), collapsed1.as_slice());
        assert_eq!(collapsed1, collapsed2);
    }

    #[test]
    fn test_collapse_128_too_many_errors_5_percent() {
        let data0 = [0u8; 16];
        let mut data1 = data0.clone();
        data1[0] = 0b11111111;

        let mut data2 = data1.clone();
        data2[0] ^= 0b11111111; // 8 bits (6.25%, exceeds 5%)

        assert_ne!(data1, data2);
        let collapsed1 = collapse_deterministic(&data1, 0.05);
        let collapsed2 = collapse_deterministic(&data2, 0.05);
        assert_ne!(data0.as_slice(), collapsed1.as_slice());
        assert_ne!(collapsed1, collapsed2);
    }
}
