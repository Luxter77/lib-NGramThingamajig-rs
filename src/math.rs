#[cfg(feature = "experiment-n_gram-skips")]
const GEOMETRIC_RATIO: f64 = 1.35;

#[cfg(feature = "experiment-n_gram-skips")]
use crate::{CTX_LEN, CHAINS_START};

pub fn build_active_orders() -> Vec<usize> {    
    #[cfg(not(feature = "experiment-n_gram-skips"))] {
        (crate::CHAINS_START..=crate::CTX_LEN).collect()
    }

    #[cfg(feature = "experiment-n_gram-skips")] {
        if crate::CTX_LEN == 0 {
            return if CHAINS_START == 0 { vec![0] } else { Vec::new() };
        }

        let mut orders = Vec::new();
        let mut cur = CTX_LEN;

        loop {
            orders.push(cur);
            if cur <= 1 {
                break;
            }
            let next = ((cur as f64) / GEOMETRIC_RATIO).round() as usize;
            cur = next.min(cur - 1).max(1);
        }

        if CHAINS_START == 0 {
            orders.push(0);
        }

        orders.reverse();
        orders
    }
}