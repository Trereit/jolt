use std::marker::PhantomData;

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

use crate::{
    field::JoltField,
    poly::dense_mlpoly::DensePolynomial,
    utils::{
        errors::ProofVerifyError,
        transcript::{AppendToTranscript, ProofTranscript},
    },
};

use super::commitment_scheme::{BatchType, CommitShape, CommitmentScheme};

#[derive(Clone)]
pub struct MockCommitScheme<F: JoltField> {
    _marker: PhantomData<F>,
}

#[derive(CanonicalSerialize, CanonicalDeserialize)]
pub struct MockCommitment<F: JoltField> {
    poly: DensePolynomial<F>,
}

impl<F: JoltField> AppendToTranscript for MockCommitment<F> {
    fn append_to_transcript(&self, label: &'static [u8], transcript: &mut ProofTranscript) {
        transcript.append_message(label, b"mocker");
    }
}

#[derive(CanonicalSerialize, CanonicalDeserialize)]
pub struct MockProof<F: JoltField> {
    opening_point: Vec<F>,
}

#[derive(CanonicalSerialize, CanonicalDeserialize)]
pub struct MockBatchedProof {}

#[derive(Clone)]
pub struct None {}

impl<F: JoltField> CommitmentScheme for MockCommitScheme<F> {
    type Field = F;
    type Setup = None;
    type Commitment = MockCommitment<F>;
    type Proof = MockProof<F>;
    type BatchedProof = MockProof<F>;

    fn setup(_shapes: &[CommitShape]) -> Self::Setup {
        None {}
    }
    fn commit(poly: &DensePolynomial<Self::Field>, _setup: &Self::Setup) -> Self::Commitment {
        MockCommitment {
            poly: poly.to_owned(),
        }
    }
    fn batch_commit(
        evals: &[&[Self::Field]],
        _gens: &Self::Setup,
        _batch_type: BatchType,
    ) -> Vec<Self::Commitment> {
        let polys: Vec<DensePolynomial<F>> = evals
            .iter()
            .map(|poly_evals| DensePolynomial::new(poly_evals.to_vec()))
            .collect();
        let commits = polys
            .into_iter()
            .map(|poly| MockCommitment { poly })
            .collect();
        commits
    }
    fn commit_slice(evals: &[Self::Field], _setup: &Self::Setup) -> Self::Commitment {
        MockCommitment {
            poly: DensePolynomial::new(evals.to_owned()),
        }
    }
    fn prove(
        _poly: &DensePolynomial<Self::Field>,
        opening_point: &[Self::Field],
        _transcript: &mut ProofTranscript,
    ) -> Self::Proof {
        MockProof {
            opening_point: opening_point.to_owned(),
        }
    }
    fn batch_prove(
        _polynomials: &[&DensePolynomial<Self::Field>],
        opening_point: &[Self::Field],
        _openings: &[Self::Field],
        _batch_type: BatchType,
        _transcript: &mut ProofTranscript,
    ) -> Self::BatchedProof {
        MockProof {
            opening_point: opening_point.to_owned(),
        }
    }

    fn verify(
        proof: &Self::Proof,
        _setup: &Self::Setup,
        _transcript: &mut ProofTranscript,
        opening_point: &[Self::Field],
        opening: &Self::Field,
        commitment: &Self::Commitment,
    ) -> Result<(), ProofVerifyError> {
        let evaluation = commitment.poly.evaluate(opening_point);
        assert_eq!(evaluation, *opening);
        assert_eq!(proof.opening_point, opening_point);
        Ok(())
    }

    fn batch_verify(
        batch_proof: &Self::BatchedProof,
        _setup: &Self::Setup,
        opening_point: &[Self::Field],
        openings: &[Self::Field],
        commitments: &[&Self::Commitment],
        _transcript: &mut ProofTranscript,
    ) -> Result<(), ProofVerifyError> {
        assert_eq!(batch_proof.opening_point, opening_point);
        assert_eq!(openings.len(), commitments.len());
        for i in 0..openings.len() {
            let evaluation = commitments[i].poly.evaluate(opening_point);
            assert_eq!(evaluation, openings[i]);
        }
        Ok(())
    }

    fn protocol_name() -> &'static [u8] {
        b"mock_commit"
    }
}
