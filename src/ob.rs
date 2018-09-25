
use super::uuid::Uuid;

use std;
use std::collections::VecDeque;
use std::fmt;
use std::ops::RangeInclusive;
use super::{Side, BookRecord, Error, Result};


/// main OrderBook structure
pub struct OrderBook {
    pub book: Vec<VecDeque<(f64, Uuid)>>,
    bid: usize,
    ask: usize,
    _match: usize
}

impl OrderBook {
    /// creates new orderbook
    pub fn new() -> Self {
        Self {
            book: vec![VecDeque::new(); super::MAX_SIZE],
            bid: std::usize::MIN,
            ask: std::usize::MAX,
            _match: 0
        }
    }

    /// get current bid
    pub fn bid(&self) -> f64 {
        self.bid as f64 / 100.0
    }

    /// get current ask
    pub fn ask(&self) -> f64 {
        self.ask as f64 / 100.0
    }

    pub fn __match(&self) -> f64 {
        self._match as f64 / 100.0
    }

    fn side(&self, range: RangeInclusive<usize>) -> Vec<f64> {
        self.book[range].iter()
            .map(|x| x.iter().map(|x| x.0).sum())
            .collect::<Vec<_>>()
    }

    /// get size of top sz bids (includes empty)
    pub fn bids(&self, sz: usize) -> Vec<f64> {
        self.side((self.bid-sz+1)..=self.bid)
    }

    /// get size of low sz bids (includes empty)
    pub fn asks(&self, sz: usize) -> Vec<f64> {
        self.side(self.ask..=self.ask+sz-1)
    }

    /// reload OrderBook from full bids and asks L3
    pub fn reload(&mut self, bids: Vec<BookRecord>, asks: Vec<BookRecord>) -> Result<()> {
        self.bid = std::usize::MIN;
        self.ask = std::usize::MAX;
        self.book.iter_mut().map(|x| *x = VecDeque::new()).count();

        bids.into_iter()
            .try_for_each(|rec| self.open(Side::Buy, rec))?;
        asks.into_iter()
            .try_for_each(|rec| self.open(Side::Sell, rec))?;
        Ok(())
    }

    fn get_idx(&self, price: f64) -> Result<usize> {
        let p_idx = (price * 100.0) as usize;
        if p_idx >= self.book.len() {
            Err(Error::Range)
        } else {
            Ok(p_idx)
        }
    }

    /// open order
    pub fn open(&mut self, side: Side, rec: BookRecord) -> Result<()> {
        let p_idx = self.get_idx(rec.price)?;
        match side {
            Side::Buy if p_idx > self.bid => self.bid = p_idx,
            Side::Sell if p_idx < self.ask => self.ask = p_idx,
            _ => (),
        }
        assert!(self.bid < self.ask);
        self.book[p_idx].push_back((rec.size, rec.id));
        Ok(())
    }

    /// match order
    pub fn _match(&mut self, price: f64, size: f64, id: Uuid) -> Result<()> {
        let p_idx = self.get_idx(price)?;

        if self.book[p_idx].is_empty() || id != self.book[p_idx][0].1 {
            return Err(Error::MatchUuid);
        }
        let mut sz = self.book[p_idx][0].0;
        sz -= size;
        if relative_eq!(sz, 0.0) {
            self.book[p_idx].pop_front();
            self.check_ask_bid(p_idx);
            self._match = p_idx;
        }
        Ok(())
    }

    pub fn test_match(&mut self, price: f64) -> Result<bool> {
        let p_idx = self.get_idx(price)?;
        if self.book[p_idx].is_empty() {
            return Err(Error::MatchUuid);
        }
        if Uuid::nil() != self.book[p_idx][0].1 {
            return Ok(false);
        }
        self.book[p_idx].pop_front();
        self.check_ask_bid(p_idx);
        Ok(true)
    }

    /// done order
    pub fn done(&mut self, price: f64, id: Uuid) -> Result<()> {
        let p_idx = self.get_idx(price)?;
        self.book[p_idx].retain(|&(_, it_id)| it_id != id);
        self.check_ask_bid(p_idx);
        Ok(())
    }

    /// change order
    pub fn change(&mut self, price: f64, new_size: f64, id: Uuid) -> Result<()> {
        let p_idx = self.get_idx(price)?;
        if new_size == 0.0 {
            self.done(price, id).unwrap_or_default();
        } else {
            self.book[p_idx].iter_mut().for_each(|(it_size, it_id)| {
                if *it_id == id {
                    *it_size = new_size;
                }
            })
        }
        Ok(())
    }

    fn check_ask_bid(&mut self, p_idx: usize) {
        if p_idx == self.bid {
            while self.book[self.bid].len() == 0 {
                self.bid -= 1;
            }
        }

        if p_idx == self.ask {
            while self.book[self.ask].len() == 0 {
                self.ask += 1;
            }
        }
    }
}

impl fmt::Display for OrderBook {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.bid == std::usize::MIN || self.ask == std::usize::MAX {
            return write!(f, "OB: empty");
        }
        let size = 20;
        let bids = self.bids(size).into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
        let asks = self.asks(size).into_iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
        let bid = self.bid as f64 / 100.0;
        let ask = self.ask as f64 / 100.0;
        write!(f, "OB: {} | {:.2}   {:.2} | {}", bids, bid, ask, asks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let mut ob = OrderBook::new();
        ob.reload(
            vec![
                BookRecord {
                    price: 3994.96,
                    size: 0.3,
                    id: Uuid::new_v4(),
                },
                BookRecord {
                    price: 3995.0,
                    size: 0.5,
                    id: Uuid::new_v4(),
                },
            ],
            vec![
                BookRecord {
                    price: 4005.0,
                    size: 0.4,
                    id: Uuid::new_v4(),
                },
                BookRecord {
                    price: 4005.02,
                    size: 0.2,
                    id: Uuid::new_v4(),
                },
            ],
        ).unwrap_or_default();

        ob.open(
            Side::Buy,
            BookRecord {
                price: 3994.96,
                size: 0.2,
                id: Uuid::new_v4(),
            },
        ).unwrap_or_default();

        let str = format!("{}", ob);
        assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.5,0,0,0,0.5 | 3995.00   4005.00 | 0.4,0,0.2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
    }

    #[test]
    fn test_match() {
        let mut ob = OrderBook::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        ob.reload(
            vec![
                BookRecord {
                    price: 3994.96,
                    size: 0.3,
                    id: id1,
                },
                BookRecord {
                    price: 3995.0,
                    size: 0.5,
                    id: id2,
                },
            ],
            vec![
                BookRecord {
                    price: 4005.0,
                    size: 0.4,
                    id: Uuid::new_v4(),
                },
                BookRecord {
                    price: 4005.02,
                    size: 0.2,
                    id: Uuid::new_v4(),
                },
            ],
        ).unwrap_or_default();

        ob.open(
            Side::Buy,
            BookRecord {
                price: 3994.96,
                size: 0.2,
                id: Uuid::new_v4(),
            },
        ).unwrap_or_default();
        ob._match(3995.0, 0.5, id2).unwrap_or_default();

        let str = format!("{}", ob);
        assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.5 | 3994.96   4005.00 | 0.4,0,0.2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
    }

    #[test]
    fn test_done() {
        let mut ob = OrderBook::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        ob.reload(
            vec![
                BookRecord {
                    price: 3994.96,
                    size: 0.3,
                    id: id1,
                },
                BookRecord {
                    price: 3995.0,
                    size: 0.5,
                    id: id2,
                },
            ],
            vec![
                BookRecord {
                    price: 4005.0,
                    size: 0.4,
                    id: Uuid::new_v4(),
                },
                BookRecord {
                    price: 4005.02,
                    size: 0.2,
                    id: Uuid::new_v4(),
                },
            ],
        ).unwrap_or_default();

        ob.done(3994.96, id1).unwrap_or_default();

        let str = format!("{}", ob);
        assert_eq!(str, "OB: 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.5 | 3995.00   4005.00 | 0.4,0,0.2,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0");
    }
}
