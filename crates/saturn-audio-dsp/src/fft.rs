// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Small radix-2 FFT for bounded offline audio analysis.

#[derive(Clone, Copy, Default)]
pub(crate) struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub fn mul(self, b: Self) -> Self {
        Self {
            re: self.re * b.re - self.im * b.im,
            im: self.re * b.im + self.im * b.re,
        }
    }
    pub fn norm(self) -> f64 {
        self.re.hypot(self.im)
    }
}

pub(crate) fn fft(data: &mut [Complex], inverse: bool) {
    let n = data.len();
    assert!(n.is_power_of_two());
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            data.swap(i, j);
        }
    }
    let mut width = 2;
    while width <= n {
        let angle = if inverse { 2.0 } else { -2.0 } * std::f64::consts::PI / width as f64;
        let step = Complex {
            re: angle.cos(),
            im: angle.sin(),
        };
        for block in data.chunks_exact_mut(width) {
            let mut phase = Complex { re: 1.0, im: 0.0 };
            for k in 0..width / 2 {
                let a = block[k];
                let b = block[k + width / 2].mul(phase);
                block[k] = Complex {
                    re: a.re + b.re,
                    im: a.im + b.im,
                };
                block[k + width / 2] = Complex {
                    re: a.re - b.re,
                    im: a.im - b.im,
                };
                phase = phase.mul(step);
            }
        }
        width *= 2;
    }
    if inverse {
        for x in data {
            x.re /= n as f64;
            x.im /= n as f64;
        }
    }
}
