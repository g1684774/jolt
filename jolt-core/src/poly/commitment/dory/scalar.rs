use ark_ec::pairing::Pairing;
use ark_ff::{Field, UniformRand};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand::thread_rng;

use crate::{
    field::JoltField,
    msm::VariableBaseMSM,
    poly::multilinear_polynomial::MultilinearPolynomial,
    utils::transcript::{AppendToTranscript, Transcript},
};

use super::{
    append_gt,
    params::SingleParam,
    vec_operations::{e, InnerProd},
    Error, G1Vec, G2Vec, Gt, PublicParams, Zr, G1, G2,
};

/// Witness over set Zr
#[derive(Clone)]
pub struct Witness<Curve>
where
    Curve: Pairing,
{
    pub u1: G1Vec<Curve>,
    pub u2: G2Vec<Curve>,
}

impl<P> Witness<P>
where
    P: Pairing,
    P::G1: VariableBaseMSM,
    P::ScalarField: JoltField,
{
    pub fn new(params: &PublicParams<P>, poly: &MultilinearPolynomial<P::ScalarField>) -> Self {
        let MultilinearPolynomial::LargeScalars(poly) = poly else {
            panic!()
        };
        let poly = poly.evals_ref();
        let v1 = params
            .g1v()
            .into_iter()
            .zip(poly.iter())
            .map(|(a, b)| a * b)
            .collect::<Vec<G1<P>>>();

        let v2 = params
            .g2v()
            .into_iter()
            .zip(poly.iter())
            .map(|(a, b)| a * b)
            .collect::<Vec<G2<P>>>();
        let v1 = v1.into();
        let v2 = v2.into();

        Self { u1: v1, u2: v2 }
    }
}

#[derive(Clone, Copy, CanonicalSerialize, CanonicalDeserialize, Debug, Default, PartialEq, Eq)]
pub struct Commitment<Curve>
where
    Curve: Pairing,
{
    pub c: Gt<Curve>,
    pub d1: Gt<Curve>,
    pub d2: Gt<Curve>,
}

impl<P> AppendToTranscript for Commitment<P>
where
    P: Pairing,
{
    fn append_to_transcript<ProofTranscript>(&self, transcript: &mut ProofTranscript)
    where
        ProofTranscript: Transcript,
    {
        append_gt(transcript, self.c);
        append_gt(transcript, self.d1);
        append_gt(transcript, self.d2);
    }
}

pub fn commit<Curve>(
    Witness { u1, u2 }: Witness<Curve>,
    public_params: &PublicParams<Curve>,
) -> Result<Commitment<Curve>, Error>
where
    Curve: Pairing,
{
    let d1 = u1.inner_prod(&public_params.g2v())?;
    let d2 = public_params.g1v().inner_prod(&u2)?;
    let c = u1.inner_prod(&u2)?;

    let commitment = Commitment { d1, d2, c };
    Ok(commitment)
}

#[derive(Clone, CanonicalDeserialize, CanonicalSerialize)]
pub struct ScalarProof<Curve>
where
    Curve: Pairing,
{
    e1: G1<Curve>,
    e2: G2<Curve>,
}

impl<Curve> ScalarProof<Curve>
where
    Curve: Pairing,
{
    pub fn new(witness: Witness<Curve>) -> Self {
        Self {
            e1: witness.u1[0],
            e2: witness.u2[0],
        }
    }

    pub fn verify(
        &self,
        pp: &SingleParam<Curve>,
        Commitment { c, d1, d2 }: &Commitment<Curve>,
    ) -> Result<bool, Error> {
        let mut rng = thread_rng();
        let d: Zr<Curve> = Zr::<Curve>::rand(&mut rng);
        let d_inv = d.inverse().ok_or(Error::CouldntInvertD)?;

        let g1 = [self.e1, pp.g1 * d].iter().sum();

        let g2 = [self.e2, pp.g2 * d_inv].iter().sum();
        let left_eq = e(g1, g2);

        let right_eq = [pp.c, *c, *d2 * d, *d1 * d_inv].iter().sum();

        Ok(left_eq == right_eq)
    }
}
