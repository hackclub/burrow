pub mod slack;
use self::grpc_defs::JwtInfo;

pub use super::{db, grpc_defs, settings::BurrowAuthServerConfig};
use anyhow::{anyhow, Result};
use jwt_simple::{
    claims::{Claims, NoCustomClaims},
    prelude::{Duration, Ed25519KeyPair, EdDSAKeyPairLike, EdDSAPublicKeyLike},
};
use serde::{Deserialize, Serialize};

pub type KeypairT = Ed25519KeyPair;

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Clone)]
pub struct OpenIdUser {
    pub sub: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenIDCustomField {
    pub name: String,
}

impl OpenIdUser {
    pub fn try_from_jwt(jwt_info: &JwtInfo, keypair: &KeypairT) -> Result<Self> {
        let claims = keypair
            .public_key()
            .verify_token::<OpenIDCustomField>(&jwt_info.jwt, None)?;
        Ok(Self {
            sub: claims.subject.ok_or(anyhow!("No Subject!"))?,
            name: claims.custom.name,
        })
    }
}

impl JwtInfo {
    fn try_from_oid(oid_user: OpenIdUser, keypair: &KeypairT) -> Result<Self> {
        let claims = Claims::with_custom_claims(
            OpenIDCustomField { name: oid_user.name },
            Duration::from_days(10),
        )
        .with_subject(oid_user.sub);
        let jwt = keypair.sign(claims)?;
        Ok(Self { jwt })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt() -> Result<()> {
        let key_pair = Ed25519KeyPair::generate();
        let sample_usr = OpenIdUser {
            sub: "Spanish".into(),
            name: "Inquisition".into(),
        };
        let encoded = JwtInfo::try_from_oid(sample_usr.clone(), &key_pair)?;
        println!("{}", encoded.jwt);
        let decoded = OpenIdUser::try_from_jwt(&encoded, &key_pair)?;
        assert_eq!(decoded, sample_usr);
        Ok(())
    }
}
