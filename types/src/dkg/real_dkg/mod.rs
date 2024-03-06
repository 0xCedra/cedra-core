// Copyright © Aptos Foundation

use crate::{
    dkg::{
        real_dkg::rounding::{DKGRounding, FAST_PATH_SECRECY_THRESHOLD},
        DKGSessionMetadata, DKGTrait,
    },
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use anyhow::{anyhow, ensure};
use aptos_crypto::{bls12381, bls12381::PrivateKey};
use aptos_dkg::{
    pvss,
    pvss::{
        traits::{Convert, Reconstructable, Transcript},
        Player,
    },
};
use num_traits::Zero;
use rand::{CryptoRng, RngCore};
use rounding::{RECONSTRUCT_THRESHOLD, SECRECY_THRESHOLD};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub mod rounding;

pub type WTrx = pvss::das::WeightedTranscript;
pub type DkgPP = <WTrx as Transcript>::PublicParameters;
pub type SSConfig = <WTrx as Transcript>::SecretSharingConfig;
pub type EncPK = <WTrx as Transcript>::EncryptPubKey;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DKGPvssConfig {
    pub epoch: u64,
    // weighted config for randomness generation
    pub wconfig: SSConfig,
    // weighted config for randomness generation in fast path
    pub fast_wconfig: Option<SSConfig>,
    // DKG public parameters
    pub pp: DkgPP,
    // DKG encryption public keys
    pub eks: Vec<EncPK>,
}

impl DKGPvssConfig {
    pub fn new(
        epoch: u64,
        wconfig: SSConfig,
        fast_wconfig: Option<SSConfig>,
        pp: DkgPP,
        eks: Vec<EncPK>,
    ) -> Self {
        Self {
            epoch,
            wconfig,
            fast_wconfig,
            pp,
            eks,
        }
    }
}

pub fn build_dkg_pvss_config(
    cur_epoch: u64,
    next_validators: &[ValidatorConsensusInfo],
) -> DKGPvssConfig {
    let validator_stakes: Vec<u64> = next_validators.iter().map(|vi| vi.voting_power).collect();

    let dkg_rounding = DKGRounding::new(
        &validator_stakes,
        SECRECY_THRESHOLD,
        RECONSTRUCT_THRESHOLD,
        Some(FAST_PATH_SECRECY_THRESHOLD),
    );

    println!(
        "[Randomness] rounding: epoch {} starts, profile = {:?}",
        cur_epoch, dkg_rounding.profile
    );

    let validator_consensus_keys: Vec<bls12381::PublicKey> = next_validators
        .iter()
        .map(|vi| vi.public_key.clone())
        .collect();

    let consensus_keys: Vec<EncPK> = validator_consensus_keys
        .iter()
        .map(|k| k.to_bytes().as_slice().try_into().unwrap())
        .collect::<Vec<_>>();

    let pp = DkgPP::default_with_bls_base();

    DKGPvssConfig::new(
        cur_epoch,
        dkg_rounding.wconfig,
        dkg_rounding.fast_wconfig,
        pp,
        consensus_keys,
    )
}

#[derive(Debug)]
pub struct RealDKG {}

#[derive(Clone, Debug)]
pub struct RealDKGPublicParams {
    pub session_metadata: DKGSessionMetadata,
    pub pvss_config: DKGPvssConfig,
    pub verifier: ValidatorVerifier,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Transcripts {
    // transcript for main path
    pub main: WTrx,
    // transcript for fast path
    pub fast: Option<WTrx>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DealtPubKeyShares {
    // dealt public key share for main path
    pub main: <WTrx as Transcript>::DealtPubKeyShare,
    // dealt public key share for fast path
    pub fast: Option<<WTrx as Transcript>::DealtPubKeyShare>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DealtSecretKeyShares {
    // dealt secret key share for main path
    pub main: <WTrx as Transcript>::DealtSecretKeyShare,
    // dealt secret key share for fast path
    pub fast: Option<<WTrx as Transcript>::DealtSecretKeyShare>,
}

impl DKGTrait for RealDKG {
    type DealerPrivateKey = <WTrx as Transcript>::SigningSecretKey;
    type DealtPubKeyShare = DealtPubKeyShares;
    type DealtSecret = <WTrx as Transcript>::DealtSecretKey;
    type DealtSecretShare = DealtSecretKeyShares;
    type InputSecret = <WTrx as Transcript>::InputSecret;
    type NewValidatorDecryptKey = <WTrx as Transcript>::DecryptPrivKey;
    type PublicParams = RealDKGPublicParams;
    type Transcript = Transcripts;

    fn new_public_params(dkg_session_metadata: &DKGSessionMetadata) -> RealDKGPublicParams {
        let pvss_config = build_dkg_pvss_config(
            dkg_session_metadata.dealer_epoch,
            &dkg_session_metadata.target_validator_consensus_infos_cloned(),
        );
        let verifier = ValidatorVerifier::new(dkg_session_metadata.dealer_consensus_infos_cloned());
        RealDKGPublicParams {
            session_metadata: dkg_session_metadata.clone(),
            pvss_config,
            verifier,
        }
    }

    fn aggregate_input_secret(secrets: Vec<Self::InputSecret>) -> Self::InputSecret {
        secrets
            .into_iter()
            .fold(<WTrx as Transcript>::InputSecret::zero(), |acc, item| {
                acc + item
            })
    }

    fn dealt_secret_from_input(
        pub_params: &Self::PublicParams,
        input: &Self::InputSecret,
    ) -> Self::DealtSecret {
        input.to(&pub_params.pvss_config.pp)
    }

    fn generate_transcript<R: CryptoRng + RngCore>(
        rng: &mut R,
        pub_params: &Self::PublicParams,
        input_secret: &Self::InputSecret,
        my_index: u64,
        sk: &Self::DealerPrivateKey,
    ) -> Self::Transcript {
        let my_index = my_index as usize;
        let my_addr = pub_params.session_metadata.dealer_validator_set[my_index].addr;
        let aux = (pub_params.session_metadata.dealer_epoch, my_addr);

        let wtrx = WTrx::deal(
            &pub_params.pvss_config.wconfig,
            &pub_params.pvss_config.pp,
            sk,
            &pub_params.pvss_config.eks,
            input_secret,
            &aux,
            &Player { id: my_index },
            rng,
        );
        // transcript for fast path
        let fast_wtrx = pub_params
            .pvss_config
            .fast_wconfig
            .as_ref()
            .map(|fast_wconfig| {
                WTrx::deal(
                    fast_wconfig,
                    &pub_params.pvss_config.pp,
                    sk,
                    &pub_params.pvss_config.eks,
                    input_secret,
                    &aux,
                    &Player { id: my_index },
                    rng,
                )
            });
        Transcripts {
            main: wtrx,
            fast: fast_wtrx,
        }
    }

    fn verify_transcript(
        params: &Self::PublicParams,
        trx: &Self::Transcript,
    ) -> anyhow::Result<()> {
        // Verify dealer indices are valid.
        let dealers = trx
            .main
            .get_dealers()
            .iter()
            .map(|player| player.id)
            .collect::<Vec<usize>>();
        let num_validators = params.session_metadata.dealer_validator_set.len();
        ensure!(
            dealers.iter().all(|id| *id < num_validators),
            "real_dkg::verify_transcript failed with invalid dealer index."
        );
        if trx.fast.is_some() {
            let _fast_dealers = trx
                .fast
                .as_ref()
                .unwrap()
                .get_dealers()
                .iter()
                .map(|player| player.id)
                .collect::<Vec<usize>>();
            ensure!(
                matches!(dealers.clone(), _fast_dealers),
                "real_dkg::verify_transcript failed with inconsistent dealer index."
            );
        }

        let all_eks = params.pvss_config.eks.clone();

        let addresses = params.verifier.get_ordered_account_addresses();
        let dealers_addresses = dealers
            .iter()
            .filter_map(|&pos| addresses.get(pos))
            .cloned()
            .collect::<Vec<_>>();

        let spks = dealers_addresses
            .iter()
            .filter_map(|author| params.verifier.get_public_key(author))
            .collect::<Vec<_>>();

        let aux = dealers_addresses
            .iter()
            .map(|address| (params.pvss_config.epoch, address))
            .collect::<Vec<_>>();

        trx.main.verify(
            &params.pvss_config.wconfig,
            &params.pvss_config.pp,
            &spks,
            &all_eks,
            &aux,
        )?;

        if trx.fast.is_some() && params.pvss_config.fast_wconfig.is_some() {
            trx.fast.as_ref().unwrap().verify(
                params.pvss_config.fast_wconfig.as_ref().unwrap(),
                &params.pvss_config.pp,
                &spks,
                &all_eks,
                &aux,
            )?;
        }

        Ok(())
    }

    fn aggregate_transcripts(
        params: &Self::PublicParams,
        accumulator: &mut Self::Transcript,
        element: Self::Transcript,
    ) {
        accumulator
            .main
            .aggregate_with(&params.pvss_config.wconfig, &element.main);
        if accumulator.fast.is_some()
            && element.fast.is_some()
            && params.pvss_config.fast_wconfig.is_some()
        {
            accumulator.fast.as_mut().unwrap().aggregate_with(
                params.pvss_config.fast_wconfig.as_ref().unwrap(),
                &element.fast.unwrap(),
            );
        }
    }

    fn decrypt_secret_share_from_transcript(
        pub_params: &Self::PublicParams,
        trx: &Self::Transcript,
        player_idx: u64,
        dk: &Self::NewValidatorDecryptKey,
    ) -> anyhow::Result<(Self::DealtSecretShare, Self::DealtPubKeyShare)> {
        let (sk, pk) = trx.main.decrypt_own_share(
            &pub_params.pvss_config.wconfig,
            &Player {
                id: player_idx as usize,
            },
            dk,
        );
        let (fast_sk, fast_pk) =
            if trx.fast.is_some() && pub_params.pvss_config.fast_wconfig.is_some() {
                let fast_trx = trx.fast.as_ref().unwrap();
                let (fast_sk, fast_pk) = fast_trx.decrypt_own_share(
                    pub_params.pvss_config.fast_wconfig.as_ref().unwrap(),
                    &Player {
                        id: player_idx as usize,
                    },
                    dk,
                );
                (Some(fast_sk), Some(fast_pk))
            } else {
                (None, None)
            };
        Ok((
            DealtSecretKeyShares {
                main: sk,
                fast: fast_sk,
            },
            DealtPubKeyShares {
                main: pk,
                fast: fast_pk,
            },
        ))
    }

    fn reconstruct_secret_from_shares(
        pub_params: &Self::PublicParams,
        input_player_share_pairs: Vec<(u64, Self::DealtSecretShare)>,
    ) -> anyhow::Result<Self::DealtSecret> {
        let player_share_pairs = input_player_share_pairs
            .clone()
            .into_iter()
            .map(|(x, y)| (Player { id: x as usize }, y.main))
            .collect();
        let reconstructed_secret = <WTrx as Transcript>::DealtSecretKey::reconstruct(
            &pub_params.pvss_config.wconfig,
            &player_share_pairs,
        );
        if input_player_share_pairs
            .clone()
            .into_iter()
            .all(|(_, y)| y.fast.is_some())
            && pub_params.pvss_config.fast_wconfig.is_some()
        {
            let fast_player_share_pairs = input_player_share_pairs
                .into_iter()
                .map(|(x, y)| (Player { id: x as usize }, y.fast.unwrap()))
                .collect();
            let fast_reconstructed_secret = <WTrx as Transcript>::DealtSecretKey::reconstruct(
                pub_params.pvss_config.fast_wconfig.as_ref().unwrap(),
                &fast_player_share_pairs,
            );
            ensure!(
                reconstructed_secret == fast_reconstructed_secret,
                "real_dkg::reconstruct_secret_from_shares failed with inconsistent dealt secrets."
            );
        }
        Ok(reconstructed_secret)
    }

    fn get_dealers(transcript: &Self::Transcript) -> BTreeSet<u64> {
        transcript
            .main
            .get_dealers()
            .into_iter()
            .map(|x| x.id as u64)
            .collect()
    }
}

pub fn maybe_dk_from_bls_sk(
    sk: &PrivateKey,
) -> anyhow::Result<<WTrx as Transcript>::DecryptPrivKey> {
    let mut bytes = sk.to_bytes(); // in big-endian
    bytes.reverse();
    <WTrx as Transcript>::DecryptPrivKey::try_from(bytes.as_slice())
        .map_err(|e| anyhow!("dk_from_bls_sk failed with dk deserialization error: {e}"))
}
