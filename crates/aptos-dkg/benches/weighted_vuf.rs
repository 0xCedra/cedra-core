// Copyright © Aptos Foundation

#![allow(clippy::needless_range_loop)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::extra_unused_type_parameters)]
#![allow(clippy::needless_borrow)]

use aptos_dkg::{
    pvss::{
        das, insecure_field,
        test_utils::{get_weighted_configs_for_benchmarking, setup_dealing, NoAux},
        traits::{SecretSharingConfig, Transcript},
        GenericWeighting, Player, WeightedConfig,
    },
    weighted_vuf::{bls, pinkas::PinkasWUF, traits::WeightedVUF},
};
use aptos_runtimes::spawn_rayon_thread_pool;
use core::iter::zip;
use criterion::{
    criterion_group, criterion_main,
    measurement::{Measurement, WallTime},
    BenchmarkGroup, Criterion,
};
use rand::{rngs::ThreadRng, thread_rng};

const BENCH_MSG: &[u8; 36] = b"some dummy message for the benchmark";

pub fn all_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("wvuf/das-pinkas-sk-in-g1");
    wvuf_benches::<das::WeightedTranscript, PinkasWUF, WallTime>(&mut group);
    group.finish();

    let mut group = c.benchmark_group("wvuf/insecure-field-bls");
    wvuf_benches::<GenericWeighting<insecure_field::Transcript>, bls::BlsWUF, WallTime>(&mut group);
    group.finish();
}

pub fn wvuf_benches<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKey = WT::DealtPubKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    M: Measurement,
>(
    group: &mut BenchmarkGroup<M>,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    let mut rng = thread_rng();

    let mut bench_cases = vec![];
    for wc in get_weighted_configs_for_benchmarking() {
        // TODO: use a lazy pattern to avoid this expensive dealing when no benchmarks are run
        let d = setup_dealing::<WT, ThreadRng>(&wc, &mut rng);

        println!(
            "Best-case subset size: {}",
            wc.get_best_case_eligible_subset_of_players(&mut rng).len()
        );
        println!(
            "Worst-case subset size: {}",
            wc.get_worst_case_eligible_subset_of_players(&mut rng).len()
        );

        println!("Dealing a {} PVSS transcript", WT::scheme_name());
        let trx = WT::deal(
            &wc,
            &d.pp,
            &d.ssks[0],
            &d.eks,
            &d.iss[0],
            &NoAux,
            &wc.get_player(0),
            &mut rng,
        );

        let vuf_pp = WVUF::PublicParameters::from(&d.pp);

        let mut sks = vec![];
        let mut pks = vec![];
        let mut deltas = vec![];
        let mut asks = vec![];
        let mut apks = vec![];
        println!(
            "Decrypting shares from {} PVSS transcript",
            WT::scheme_name()
        );
        for i in 0..wc.get_total_num_players() {
            let (sk, pk) = trx.decrypt_own_share(&wc, &wc.get_player(i), &d.dks[i]);
            let (ask, apk) = WVUF::augment_key_pair(&vuf_pp, sk.clone(), pk.clone(), &mut rng);

            sks.push(sk);
            pks.push(pk);
            deltas.push(WVUF::get_public_delta(&apk).clone());
            asks.push(ask);
            apks.push(apk);
        }
        println!();

        bench_cases.push((wc, vuf_pp, d.dsk, d.dpk, sks, pks, asks, apks, deltas));
    }

    for (wc, vuf_pp, sk, pk, sks, pks, asks, apks, deltas) in bench_cases {
        wvuf_augment_random_keypair::<WT, WVUF, ThreadRng, M>(
            &wc, &vuf_pp, &sks, &pks, group, &mut rng,
        );

        wvuf_augment_all_pubkeys::<WT, WVUF, ThreadRng, M>(&wc, &vuf_pp, &pks, &deltas, group);

        wvuf_augment_random_pubkey::<WT, WVUF, ThreadRng, M>(
            &wc, &vuf_pp, &pks, &deltas, group, &mut rng,
        );

        wvuf_create_share::<WT, WVUF, ThreadRng, M>(&wc, &asks, group, &mut rng);

        wvuf_verify_share::<WT, WVUF, ThreadRng, M>(&wc, &vuf_pp, &asks, &apks, group, &mut rng);

        let bc: Vec<(fn(&WeightedConfig, &mut ThreadRng) -> Vec<Player>, String)> = vec![
            (
                WeightedConfig::get_random_eligible_subset_of_players,
                "random".to_string(),
            ),
            (
                WeightedConfig::get_best_case_eligible_subset_of_players,
                "best_case".to_string(),
            ),
            (
                WeightedConfig::get_worst_case_eligible_subset_of_players,
                "worst_case".to_string(),
            ),
        ];

        for (pick_subset_fn, subset_type) in bc {
            // best-case aggregation times (pick players with largest weights)
            wvuf_aggregate_shares::<WT, WVUF, ThreadRng, M>(
                &wc,
                &asks,
                &apks,
                group,
                &mut rng,
                pick_subset_fn,
                &subset_type,
            );

            wvuf_verify_proof::<WT, WVUF, ThreadRng, M>(
                &wc,
                &vuf_pp,
                &pk,
                &asks,
                &apks,
                group,
                &mut rng,
                pick_subset_fn,
                &subset_type,
            );

            for num_threads in [1, 2, 4, 8, 16, 32] {
                wvuf_derive_eval::<WT, WVUF, ThreadRng, M>(
                    &wc,
                    &vuf_pp,
                    &asks,
                    &apks,
                    group,
                    &mut rng,
                    pick_subset_fn,
                    &subset_type,
                    num_threads,
                );
            }
        }

        wvuf_eval::<WT, WVUF, M>(&wc, &sk, group);
    }
}

fn wvuf_augment_random_keypair<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    sks: &Vec<WT::DealtSecretKeyShare>,
    pks: &Vec<WT::DealtPubKeyShare>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("augment_random_keypair/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                // Ugh, borrow checker...
                let id = wc.get_random_player(&mut thread_rng()).id;
                (sks[id].clone(), pks[id].clone())
            },
            |(sk, pk)| WVUF::augment_key_pair(vuf_pp, sk, pk, rng),
        )
    });
}

fn wvuf_augment_all_pubkeys<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    pks: &Vec<WVUF::PubKeyShare>,
    deltas: &Vec<WVUF::Delta>,
    group: &mut BenchmarkGroup<M>,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    assert_eq!(pks.len(), wc.get_total_num_players());
    assert_eq!(pks.len(), deltas.len());
    group.bench_function(format!("augment_all_pubkeys/{}", wc), move |b| {
        b.iter(|| {
            for (pk, delta) in zip(pks, deltas) {
                WVUF::augment_pubkey(vuf_pp, pk.clone(), delta.clone())
                    .expect("augmentation should have succeeded");
            }
        })
    });
}

fn wvuf_augment_random_pubkey<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    pks: &Vec<WVUF::PubKeyShare>,
    deltas: &Vec<WVUF::Delta>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("augment_random_pubkey/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                // Ugh, borrow checker...
                let id = wc.get_random_player(rng).id;
                let pk = pks[id].clone();
                let delta = deltas[id].clone();

                (pk, delta)
            },
            |(pk, delta)| WVUF::augment_pubkey(vuf_pp, pk, delta),
        )
    });
}

fn wvuf_create_share<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    wc: &WeightedConfig,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("create_share/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                let player = wc.get_random_player(rng);
                &asks[player.id]
            },
            |ask| WVUF::create_share(ask, BENCH_MSG),
        )
    });
}

fn wvuf_verify_share<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("verify_share/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                let player = wc.get_random_player(rng);
                let ask = &asks[player.id];

                (WVUF::create_share(ask, BENCH_MSG), &apks[player.id])
            },
            |(proof, apk)| WVUF::verify_share(vuf_pp, apk, BENCH_MSG, &proof),
        )
    });
}

fn wvuf_aggregate_shares<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKey = WT::DealtPubKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
    subset_type: &String,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(
        format!("aggregate_shares/{}-subset/{}", subset_type, wc),
        move |b| {
            b.iter_with_setup(
                || get_apks_and_proofs::<WT, WVUF, R>(&wc, &asks, apks, rng, pick_subset_fn),
                |apks_and_proofs| {
                    WVUF::aggregate_shares(&wc, apks_and_proofs.as_slice());
                },
            )
        },
    );
}

fn wvuf_verify_proof<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKey = WT::DealtPubKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    pp: &WVUF::PublicParameters,
    pk: &WVUF::PubKey,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
    subset_type: &String,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(
        format!("verify_proof/{}-subset/{}", subset_type, wc),
        move |b| {
            b.iter_with_setup(
                || {
                    let apks_and_proofs =
                        get_apks_and_proofs::<WT, WVUF, R>(&wc, &asks, apks, rng, pick_subset_fn);
                    WVUF::aggregate_shares(&wc, apks_and_proofs.as_slice())
                },
                |proof| {
                    let apks = apks
                        .iter()
                        .map(|apk| Some(apk.clone()))
                        .collect::<Vec<Option<WVUF::AugmentedPubKeyShare>>>();
                    assert!(WVUF::verify_proof(pp, pk, apks.as_slice(), BENCH_MSG, &proof).is_ok())
                },
            )
        },
    );
}

fn wvuf_derive_eval<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKey = WT::DealtPubKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    pp: &WVUF::PublicParameters,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
    subset_type: &String,
    num_threads: usize,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    let pool = spawn_rayon_thread_pool("bench-wvuf".to_string(), Some(num_threads));

    group.bench_function(
        format!(
            "derive_eval/{}-thread/{}-subset/{}",
            num_threads, subset_type, wc
        ),
        move |b| {
            b.iter_with_setup(
                || {
                    let apks_and_proofs =
                        get_apks_and_proofs::<WT, WVUF, R>(&wc, &asks, apks, rng, pick_subset_fn);
                    WVUF::aggregate_shares(&wc, apks_and_proofs.as_slice())
                },
                |proof| {
                    let apks = apks
                        .iter()
                        .map(|apk| Some(apk.clone()))
                        .collect::<Vec<Option<WVUF::AugmentedPubKeyShare>>>();
                    assert!(
                        WVUF::derive_eval(wc, pp, BENCH_MSG, apks.as_slice(), &proof, &pool)
                            .is_ok()
                    )
                },
            )
        },
    );
}

fn get_apks_and_proofs<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKey = WT::DealtPubKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
) -> Vec<(Player, WVUF::AugmentedPubKeyShare, WVUF::ProofShare)> {
    let players = pick_subset_fn(wc, rng);

    players
        .iter()
        .map(|p| {
            (
                *p,
                apks[p.id].clone(),
                WVUF::create_share(&asks[p.id], BENCH_MSG),
            )
        })
        .collect::<Vec<(Player, WVUF::AugmentedPubKeyShare, WVUF::ProofShare)>>()
}

fn wvuf_eval<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    M: Measurement,
>(
    wc: &WeightedConfig,
    sk: &WVUF::SecretKey,
    group: &mut BenchmarkGroup<M>,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("eval/{}", wc), move |b| {
        b.iter_with_setup(|| {}, |_| WVUF::eval(sk, BENCH_MSG))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    //config = Criterion::default();
    targets = all_groups);
criterion_main!(benches);
