#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

use bzip2::read::BzDecoder;
use bzip2::Compression;
use histogram::Histogram;
use rand::{prelude::StdRng, seq::SliceRandom, thread_rng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, io::BufRead};
#[derive(Debug, PartialEq, Clone)]
pub enum Card {
    ACE,
    TWO,
    THREE,
    FOUR,
    FIVE,
    SIX,
    SEVEN,
    EIGHT,
    NINE,
    TEN,
    JOKER,
}

impl Card {
    fn symbol(&self) -> &'static str {
        match self {
            Card::ACE => "A",
            Card::TWO => "2",
            Card::THREE => "3",
            Card::FOUR => "4",
            Card::FIVE => "5",
            Card::SIX => "6",
            Card::SEVEN => "7",
            Card::EIGHT => "8",
            Card::NINE => "9",
            Card::TEN => "0",
            Card::JOKER => "J",
        }
    }

    fn value(&self) -> i64 {
        match self {
            Card::ACE => 1,
            Card::TWO => 2,
            Card::THREE => 3,
            Card::FOUR => 4,
            Card::FIVE => 5,
            Card::SIX => 6,
            Card::SEVEN => 7,
            Card::EIGHT => 8,
            Card::NINE => 9,
            Card::TEN => 10,
            Card::JOKER => 0,
        }
    }
}

pub struct CardList {
    ccards: Vec<Card>,
}

impl CardList {
    fn new() -> CardList {
        CardList { ccards: Vec::new() }
    }

    fn draw_top_card(&mut self) -> Card {
        self.ccards.pop().unwrap()
    }

    fn place_card_on_top(&mut self, c: Card) {
        self.ccards.push(c)
    }

    fn remove_card_of_type(&mut self, c: Card) {
        self.ccards
            .remove(self.ccards.iter().position(|x| *x == c).unwrap());
    }

    fn is_empty(&self) -> bool {
        self.ccards.len() == 0
    }

    fn score_hand(&self) -> i64 {
        self.ccards.iter().map(|c| c.value()).sum()
    }
}

impl std::fmt::Debug for CardList {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let symbols: Vec<&str> = self.ccards.iter().map(|x| x.symbol()).collect();
        write!(f, "[{}]", symbols.join(""))
    }
}
#[derive(Debug)]
pub enum Buy {
    JackWith(Card),
    QueenWith(Card),
    KingWith(Card),
}

type BuyPolicyType = Box<dyn Fn(&Game, usize) -> Option<Buy>>;

pub struct Player<'a> {
    pub name: String,
    pub hand: CardList,
    pub jacks: i64,
    pub queens: i64,
    pub buy_policy: &'a BuyPolicyType,
    pub reorder_policy: fn(&mut Game, usize),
}

impl<'a> std::fmt::Debug for Player<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}-{:?}-J:{}-Q:{}",
            self.name, self.hand, self.jacks, self.queens
        )
    }
}

#[derive(Debug)]
pub struct Game<'a> {
    pub players: Vec<Player<'a>>,
    pub unbought_kings: i64,
    pub remaining_cards: CardList,
}

fn cheapest_card_that_can_pay_x(cards: &CardList, cost: i64) -> Option<Card> {
    let mut best: Option<Card> = None;
    for c in &cards.ccards {
        let new = c.clone();
        if new.value() >= cost {
            match &best {
                Some(old) => {
                    if new.value() < old.value() {
                        best = Some(new)
                    }
                }
                None => best = Some(new),
            }
        }
    }
    best
}

fn IDLE_POLICY(game: &Game, current_player_idx: usize) -> Option<Buy> {
    let _current_player = &game.players[current_player_idx];
    // println!("# {} policy: idles", current_player.name);
    None
}

fn KING_BUYER_POLICY(game: &Game, current_player_idx: usize) -> Option<Buy> {
    let current_player = &game.players[current_player_idx];
    if game.unbought_kings == 0 {
        // println!(
        //     "# {} policy: already happy since all kings are bought",
        //     current_player.name
        // );
        return None;
    }
    // println!(
    //     "# {} policy: wants to buy a king with cards: {:?}",
    //     current_player.name, current_player.hand
    // );
    let next_king_cost = 5 - game.unbought_kings;
    match cheapest_card_that_can_pay_x(&current_player.hand, next_king_cost) {
        None => None,
        Some(card) => Some(Buy::KingWith(card)),
    }
}

fn JACK_BUYER_POLICY(game: &Game, current_player_idx: usize) -> Option<Buy> {
    let jacks_bought: i64 = game.players.iter().map(|p| p.jacks).sum();
    let current_player = &game.players[current_player_idx];
    if jacks_bought == 4 {
        // println!("# {} policy: no jacks left to buy", current_player.name);
        return None;
    }
    // println!(
    //     "# {} policy: wants to buy a jack with cards: {:?}",
    //     current_player.name, current_player.hand
    // );
    let next_jack_cost = jacks_bought + 1;
    match cheapest_card_that_can_pay_x(&current_player.hand, next_jack_cost) {
        None => None,
        Some(card) => Some(Buy::JackWith(card)),
    }
}

fn ONE_QUEEN_THEN_IDLE(game: &Game, current_player_idx: usize) -> Option<Buy> {
    let queens_bought: i64 = game.players.iter().map(|p| p.queens).sum();
    let current_player = &game.players[current_player_idx];
    if current_player.queens > 0 {
        return None;
    } else if queens_bought == 4 {
        return None;
    } else {
    }
    let next_queen_cost = queens_bought + 1;
    match cheapest_card_that_can_pay_x(&current_player.hand, next_queen_cost) {
        None => None,
        Some(card) => Some(Buy::QueenWith(card)),
    }
}

fn DEFAULT_REORDER_POLICY(game: &mut Game, current_player_idx: usize) {
    if game.players[current_player_idx].queens == 0 {
        return;
    }

    let mut cards = Vec::new();
    for _ in 0..game.players[current_player_idx].queens + 1 {
        if !game.remaining_cards.is_empty() {
            cards.push(game.remaining_cards.draw_top_card());
        }
    }

    cards.sort_by(|a, b| a.value().cmp(&b.value()));

    for c in cards {
        game.remaining_cards.place_card_on_top(c);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuyablePiece {
    JACK,
    QUEEN,
    KING,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BuyPolicyItem {
    pub piece_type: BuyablePiece,
    pub piece_num: i64,
    pub budget: i64,
}

impl std::fmt::Debug for BuyPolicyItem {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:?}#{}@{}",
            self.piece_type, self.piece_num, self.budget
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyPolicyConfig {
    pub priorities: Vec<BuyPolicyItem>,
}

fn next_jack_cost(game: &Game) -> i64 {
    let jacks_bought: i64 = game.players.iter().map(|p| p.jacks).sum();
    if jacks_bought == 4 {
        return 1000;
    } else {
        return jacks_bought + 1;
    }
}

fn next_queen_cost(game: &Game) -> i64 {
    let queens_bought: i64 = game.players.iter().map(|p| p.queens).sum();
    if queens_bought == 4 {
        return 1000;
    } else {
        return queens_bought + 1;
    }
}

fn next_king_cost(game: &Game) -> i64 {
    let kings_bought: i64 = 4 - game.unbought_kings;
    if kings_bought == 4 {
        return 1000;
    } else {
        return kings_bought + 1;
    }
}

fn COSTED_POLICY(game: &Game, current_player_idx: usize, policy: &BuyPolicyConfig) -> Option<Buy> {
    let j_cost = next_jack_cost(&game);
    let q_cost = next_queen_cost(&game);
    let k_cost = next_king_cost(&game);
    let kings_bought = 4 - game.unbought_kings;
    let current_player = &game.players[current_player_idx];

    let (jack_cost, jack_opt) = match cheapest_card_that_can_pay_x(&current_player.hand, j_cost) {
        None => (1000, None),
        Some(card) => (card.value(), Some(Buy::JackWith(card))),
    };
    let (queen_cost, queen_opt) = match cheapest_card_that_can_pay_x(&current_player.hand, q_cost) {
        None => (1000, None),
        Some(card) => (card.value(), Some(Buy::QueenWith(card))),
    };
    let (king_cost, king_opt) = match cheapest_card_that_can_pay_x(&current_player.hand, k_cost) {
        None => (1000, None),
        Some(card) => (card.value(), Some(Buy::KingWith(card))),
    };

    for p in &policy.priorities {
        match &p.piece_type {
            BuyablePiece::JACK => {
                if p.piece_num != current_player.jacks + 1 {
                } else if p.budget < jack_cost {
                } else {
                    return jack_opt;
                }
            }
            BuyablePiece::QUEEN => {
                if p.piece_num != current_player.queens + 1 {
                } else if p.budget < queen_cost {
                } else {
                    return queen_opt;
                }
            }
            BuyablePiece::KING => {
                if p.piece_num != kings_bought + 1 {
                } else if p.budget < king_cost {
                } else {
                    return king_opt;
                }
            }
        };
    }

    None
}

impl<'a> Player<'a> {
    fn new(name: String, buy_policy: &BuyPolicyType) -> Player {
        Player {
            name,
            hand: CardList::new(),
            jacks: 0,
            queens: 0,
            buy_policy,
            reorder_policy: DEFAULT_REORDER_POLICY,
        }
    }
}

fn make_random_costed_policy<R: Rng + ?Sized>(rng: &mut R) -> Vec<BuyPolicyItem> {
    let mut policy = Vec::new();
    let mut rand_budget = || -> i64 { rng.gen_range(0..11) };
    for i in 1..5 {
        policy.push(BuyPolicyItem {
            piece_type: BuyablePiece::JACK,
            piece_num: i,
            budget: rand_budget(),
        });
        policy.push(BuyPolicyItem {
            piece_type: BuyablePiece::QUEEN,
            piece_num: i,
            budget: rand_budget(),
        });
        policy.push(BuyPolicyItem {
            piece_type: BuyablePiece::KING,
            piece_num: i,
            budget: rand_budget(),
        });
    }
    policy.shuffle(rng);
    return policy;
}

fn init_deck<R: Rng + ?Sized>(rng: &mut R) -> CardList {
    let mut cards = Vec::new();
    cards.push(Card::JOKER);
    for _ in 0..4 {
        cards.push(Card::ACE);
        cards.push(Card::TWO);
        cards.push(Card::THREE);
        cards.push(Card::FOUR);
        cards.push(Card::FIVE);
        cards.push(Card::SIX);
        cards.push(Card::SEVEN);
        cards.push(Card::EIGHT);
        cards.push(Card::NINE);
        cards.push(Card::TEN);
    }
    while cards.iter().position(|x| *x == Card::JOKER).unwrap() <= 13 {
        cards.shuffle(rng);
    }
    cards.reverse();
    let mut deck = CardList::new();
    deck.ccards = cards;
    return deck;
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum GameResult {
    Paperclips,
    Draw,
    WinnerNamed(String),
}

fn verbose_play_game(mut game: &mut Game) -> GameResult {
    let mut current_player_idx = 0;
    'outer: loop {
        // println!("| state: {:?}", game);
        {
            let current_player = &mut game.players[current_player_idx];
            (current_player.reorder_policy)(&mut game, current_player_idx);
        }
        // println!("| reord: {:?}", game);

        {
            let current_player = &mut game.players[current_player_idx];
            let cards_to_draw = 1 + current_player.jacks;
            for _ in 0..cards_to_draw {
                let draw = game.remaining_cards.draw_top_card();
                // println!("player {} drew: {:?}", current_player.name, draw);
                if draw == Card::JOKER {
                    // println!("> and we have GAI");
                    break 'outer;
                } else if draw == Card::ACE && game.unbought_kings > 0 {
                    // println!("> player {} discards their hand", current_player.name);
                    current_player.hand = CardList::new();
                }

                current_player.hand.place_card_on_top(draw.clone());
            }
        }

        let current_player = &game.players[current_player_idx];
        let action = (current_player.buy_policy)(&game, current_player_idx);

        let current_player = &mut game.players[current_player_idx];
        // println!("! player {} action: {:?}", current_player.name, action);

        match action {
            Some(Buy::KingWith(card)) => {
                current_player.hand.remove_card_of_type(card);
                game.unbought_kings -= 1;
            }
            Some(Buy::JackWith(card)) => {
                current_player.hand.remove_card_of_type(card);
                current_player.jacks += 1;
            }
            Some(Buy::QueenWith(card)) => {
                current_player.hand.remove_card_of_type(card);
                current_player.queens += 1;
            }
            _ => {}
        }

        current_player_idx += 1;
        if current_player_idx == game.players.len() {
            current_player_idx = 0;
        }
    }
    if game.unbought_kings > 0 {
        // println!("| state: {:?}", game);
        // println!("the GAI turned humans into paperclips");
        GameResult::Paperclips
    } else {
        // println!("| state: {:?}", game);
        // println!("scores:");
        let mut highest_score = -1000;
        let mut num_highest_score = -1;
        for player in &game.players {
            let score: i64 = player.hand.score_hand();
            // println!("  {}: {}", player.name, score);
            if score > highest_score {
                highest_score = score;
                num_highest_score = 0;
            }
            if score == highest_score {
                num_highest_score += 1;
            }
        }
        if num_highest_score >= 2 {
            GameResult::Draw
        } else {
            let mut result = GameResult::Draw;
            for player in &game.players {
                let score: i64 = player.hand.score_hand();
                if score == highest_score {
                    result = GameResult::WinnerNamed(player.name.clone());
                }
            }
            result
        }
    }
}

// fn play_once() {
//     let p1 = Player::new(String::from("p1onequeen"), &Box::new(ONE_QUEEN_THEN_IDLE));
//     let p2 = Player::new(String::from("p2kingbuyer"), &Box::new(KING_BUYER_POLICY));
//     let mut game = Game {
//         players: vec![p1, p2],
//         unbought_kings: 4,
//         remaining_cards: init_deck(),
//     };
//     println!("Hello, world: {:?}", game);
//     let result = verbose_play_game(&mut game);
//     println!("game result: {:?}", result);
// }

fn mk_player_from_config(
    base_config: BuyPolicyConfig,
    all_kings_config: BuyPolicyConfig,
) -> BuyPolicyType {
    return Box::new(move |game: &Game, current_player_idx: usize| {
        if game.unbought_kings == 0 {
            COSTED_POLICY(&game, current_player_idx, &all_kings_config)
        } else {
            COSTED_POLICY(&game, current_player_idx, &base_config)
        }
    });
}

fn mk_random_player<R: Rng + ?Sized>(rng: &mut R) -> (BuyPolicyType, BuyPolicyConfig) {
    let policy = BuyPolicyConfig {
        priorities: make_random_costed_policy(rng),
    };
    let player = mk_player_from_config(policy.clone(), policy.clone());
    return (player, policy);
}

fn mk_random_player_all_kings<R: Rng + ?Sized>(
    rng: &mut R,
) -> (BuyPolicyType, BuyPolicyConfig, BuyPolicyConfig) {
    let base_policy = BuyPolicyConfig {
        priorities: make_random_costed_policy(rng),
    };
    let kings_policy = BuyPolicyConfig {
        priorities: make_random_costed_policy(rng),
    };
    let player = mk_player_from_config(base_policy.clone(), kings_policy.clone());
    return (player, base_policy, kings_policy);
}

fn play_policies_against_each_other<R: Rng + ?Sized>(
    rng: &mut R,
    a: &BuyPolicyType,
    b: &BuyPolicyType,
) -> f64 {
    let mut play = |x: &BuyPolicyType, y: &BuyPolicyType| -> u64 {
        let result = verbose_play_game(&mut Game {
            players: vec![
                Player::new(String::from("first"), x),
                Player::new(String::from("second"), y),
            ],
            unbought_kings: 4,
            remaining_cards: init_deck(rng),
        });
        match result {
            GameResult::WinnerNamed(name) => {
                if name == "first" {
                    1
                } else {
                    2
                }
            }
            _ => 0,
        }
    };
    let mut score = 0.0;
    if play(a, b) == 1 {
        score += 0.5;
    }
    if play(b, a) == 2 {
        score += 0.5;
    }
    score
}

fn eval_policy_against_random_policy<R: Rng + ?Sized>(
    rng: &mut R,
    times: i64,
    policy: &BuyPolicyType,
) -> f64 {
    let mut all_results = std::collections::HashMap::new();
    for _ in 0..times {
        let (random, _) = mk_random_player(rng);
        let players = if rng.gen_bool(0.5) {
            vec![
                Player::new(String::from("policy"), policy),
                Player::new(String::from("random"), &random),
            ]
        } else {
            vec![
                Player::new(String::from("random"), &random),
                Player::new(String::from("policy"), policy),
            ]
        };
        let result = verbose_play_game(&mut Game {
            players,
            unbought_kings: 4,
            remaining_cards: init_deck(rng),
        });
        // println!("game result: {:?}", result);
        *all_results.entry(result).or_insert(0) += 1;
    }
    // println!("overall results: {:?}", all_results);
    let policy_score = *all_results
        .get(&GameResult::WinnerNamed(String::from("policy")))
        .unwrap_or(&0);
    // println!("p1: {:?}", policy_score);
    return policy_score as f64 / times as f64;
}

// fn play_many() {
//     let mut all_results = std::collections::HashMap::new();
//     let mk_player = || {
//         let policy = BuyPolicyConfig {
//             priorities: make_random_costed_policy(),
//         };
//         let f = move |game: &Game, current_player_idx: usize| {
//             COSTED_POLICY(&game, current_player_idx, &policy)
//         };
//         return f;
//     };
//     for _ in 0..100_000 {
//         let result = verbose_play_game(&mut Game {
//             players: vec![
//                 Player::new(String::from("p1idle"), Box::new(IDLE_POLICY)),
//                 // Player::new(String::from("p1onequeen"), ONE_QUEEN_THEN_IDLE),
//                 // Player::new(String::from("p2onequeen"), ONE_QUEEN_THEN_IDLE),
//                 // Player::new(String::from("p3kingbuyer"), KING_BUYER_POLICY),
//                 // Player::new(String::from("p3jackbuyer"), JACK_BUYER_POLICY),
//                 Player::new(String::from("p2random"), Box::new(mk_player())),
//             ],
//             unbought_kings: 4,
//             remaining_cards: init_deck(),
//         });
//         // println!("game result: {:?}", result);
//         *all_results.entry(result).or_insert(0) += 1;
//     }
//     println!("overall results: {:?}", all_results);
// }

// fn find_good_random_policy() {
//     let mut best_score = -100.0;
//     // let mut best_policy = None;
//     loop {
//         let (policy, config) = mk_random_player();
//         let fast_score = eval_policy_against_random_policy(10_000, &policy);
//         if fast_score < best_score - 0.1 {
//             continue;
//         }
//         let score = eval_policy_against_random_policy(1_000_000, &policy);
//         if score > best_score {
//             println!("new best policy with score {}: {:?}", score, config);
//             best_score = score;
//             // best_policy = Some(config);
//         }
//     }
// }

// fn find_ok_random_policies() {
//     loop {
//         let (policy, config) = mk_random_player();
//         let fast_score = eval_policy_against_random_policy(10_000, &policy);
//         if fast_score < 0.3 {
//             continue;
//         }
//         let score = eval_policy_against_random_policy(1_000_000, &policy);
//         if score > 0.4 {
//             println!(
//                 "good policy with score {} | {}: {:?}",
//                 score, fast_score, config
//             );
//         }
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvalResult {
    pub policy: BuyPolicyConfig,
    pub all_kings_policy: Option<BuyPolicyConfig>,
    pub score: f64,
    pub times: i64,
}

// p25: 168
// p50: 223
// p75: 291
// p80: 312
// p90: 374
// p92.5: 397
// p95: 425
// p97.5: 464
// p99: 505
// p100: 703

// fn find_fast_random_policies() {
//     let mut histogram = Histogram::new();

//     loop {
//         let (random_policy, random_policy_config) = mk_random_player();
//         let times = 10_000;
//         let fast_score = eval_policy_against_random_policy(times, &random_policy);
//         histogram
//             .increment((fast_score * 1000.0) as u64)
//             .expect("failed to register fast_score in histogram");
//         if fast_score > 0.3 {
//             eprintln!(
//                 "policy with fast-score {}: {:?}",
//                 fast_score, random_policy_config
//             );
//             println!(
//                 "{}",
//                 serde_json::to_string(&PolicyEvalResult {
//                     policy: random_policy_config,
//                     all_kings_policy: None,
//                     score: fast_score,
//                     times,
//                 })
//                 .unwrap()
//             );
//         }
//         if fast_score >= 0.5 {
//             for percentile in [25.0, 50.0, 75.0, 80.0, 90.0, 92.5, 95.0, 97.5, 99.0, 100.0].iter() {
//                 eprintln!(
//                     "p{}: {}",
//                     percentile,
//                     histogram.percentile(*percentile).unwrap(),
//                 );
//             }
//         }
//     }
// }

// == single policy ==
// p25: 197
// p50: 254
// p75: 313
// p80: 331
// p90: 374
// p92.5: 391
// p95: 413
// p97.5: 446
// p99: 479
// p100: 610

// == base + kings policy ==
// p25: 211
// p50: 263
// p75: 322
// p80: 336
// p90: 372
// p92.5: 388
// p95: 405
// p97.5: 433
// p99: 480
// p100: 628

// == base + kings policy vs hard meta ==
// p25: 196
// p50: 257
// p75: 316
// p80: 330
// p90: 369
// p92.5: 383
// p95: 402
// p97.5: 432
// p99: 463
// p100: 583

// fn play_policies_against_each_other(a: &BuyPolicyType, b: &BuyPolicyType) -> f64 {
// 204866 policies >= 30%, 10444 policies >= 50%
// 20% random, 60% 30+, 20% 50+
// 60% => 204866
// 20% => ~68288 => 6x policy_50

fn eval_against_policy_set<R: Rng + ?Sized>(
    rng: &mut R,
    x: &BuyPolicyType,
    tests: &Vec<BuyPolicyType>,
) -> f64 {
    let mut score = 0.0;
    let mut max = 0.0;
    for t in tests {
        score += play_policies_against_each_other(rng, x, t);
        max += 1.0;
    }
    return score / max;
}

type StoredPolicy = Vec<Box<dyn Fn(&Game, usize) -> Option<Buy>>>;

pub fn read_policies() -> (StoredPolicy, StoredPolicy) {
    let file =
        std::fs::File::open("records/policies_above_0.3_score_vs_random_policy.jsonl.bz2").unwrap();
    let decompressor = BzDecoder::new(file);

    let mut policies_above_30 = vec![];
    let mut policies_above_50 = vec![];
    {
        for line_or in std::io::BufReader::new(decompressor).lines() {
            let line = line_or.unwrap();
            let result: PolicyEvalResult = serde_json::from_str(&line).unwrap();
            // eprintln!("score {} from policy: {:?}", result.score, result.policy);
            policies_above_30.push(mk_player_from_config(
                result.policy.clone(),
                result.policy.clone(),
            ));
            if result.score >= 0.5 {
                policies_above_50.push(mk_player_from_config(
                    result.policy.clone(),
                    result.policy.clone(),
                ));
            }
        }
    }
    (policies_above_30, policies_above_50)
}

pub fn check_meta_policies() {
    let mut rng = thread_rng();
    let mut histogram = Histogram::new();

    let (policies_above_30, policies_above_50) = read_policies();

    eprintln!(
        "{} policies >= 30%, {} policies >= 50%",
        policies_above_30.len(),
        policies_above_50.len()
    );

    loop {
        // let (random_policy, random_policy_config) = mk_random_player();
        let (random_policy, base_config, kings_config) =
            mk_random_player_all_kings(&mut thread_rng());
        let random_score = eval_policy_against_random_policy(&mut rng, 140_000, &random_policy);
        let thirty_score = eval_against_policy_set(&mut rng, &random_policy, &policies_above_30);
        let fifty_score = eval_against_policy_set(&mut rng, &random_policy, &policies_above_50);
        // let combined_score = random_score * 0.20 + thirty_score * 0.60 + fifty_score * 0.20;
        let combined_score = random_score * 0.10 + thirty_score * 0.20 + fifty_score * 0.70;
        histogram
            .increment((combined_score * 1000.0) as u64)
            .expect("failed to register fast_score in histogram");
        eprintln!(
            "policy with combined-score {}: {:?} then {:?}",
            combined_score, base_config, kings_config
        );
        println!(
            "{}",
            serde_json::to_string(&PolicyEvalResult {
                policy: base_config,
                all_kings_policy: Some(kings_config),
                score: combined_score,
                times: 0,
            })
            .unwrap()
        );
        if combined_score >= 0.1 {
            for percentile in [25.0, 50.0, 75.0, 80.0, 90.0, 92.5, 95.0, 97.5, 99.0, 100.0].iter() {
                eprintln!(
                    "p{}: {}",
                    percentile,
                    histogram.percentile(*percentile).unwrap(),
                );
            }
            eprintln!("  =====  ");
        }
    }
}

pub fn run_profiling_test(policies: i64, times: i64) {
    // let mut rng = thread_rng();
    let mut rng = StdRng::seed_from_u64(123);
    let mut total_scores = 0.0;
    for _i in 0..policies {
        let (random_policy, _base_config, _kings_config) = mk_random_player_all_kings(&mut rng);
        let random_score = eval_policy_against_random_policy(&mut rng, times, &random_policy);
        total_scores += random_score;
    }
    eprintln!("total score: {}", total_scores);
}
fn main() {
    // println!(
    //     "IDLE_POLICY:         {:?}",
    //     eval_policy(100_000, &Box::new(IDLE_POLICY))
    // );
    // println!(
    //     "ONE_QUEEN_THEN_IDLE: {:?}",
    //     eval_policy(100_000, &Box::new(ONE_QUEEN_THEN_IDLE))
    // );
    // println!(
    //     "KING_BUYER_POLICY:   {:?}",
    //     eval_policy(100_000, &Box::new(KING_BUYER_POLICY))
    // );
    // println!(
    //     "JACK_BUYER_POLICY:   {:?}",
    //     eval_policy(100_000, &Box::new(JACK_BUYER_POLICY))
    // );
    // find_good_random_policy();
    // find_ok_random_policies();
    // find_fast_random_policies();
    // check_meta_policies();
    run_profiling_test(5, 10_000);
}
