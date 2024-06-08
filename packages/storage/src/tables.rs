use ports::types::{BlockSubmission, StateFragment, StateSubmission};

macro_rules! bail {
    ($msg: literal, $($args: expr),*) => {
        return Err(Self::Error::Conversion(format!($msg, $($args),*)));
    };
}

#[derive(sqlx::FromRow)]
pub struct L1FuelBlockSubmission {
    pub fuel_block_hash: Vec<u8>,
    pub fuel_block_height: i64,
    pub completed: bool,
    pub submittal_height: i64,
}

impl TryFrom<L1FuelBlockSubmission> for BlockSubmission {
    type Error = crate::error::Error;

    fn try_from(value: L1FuelBlockSubmission) -> Result<Self, Self::Error> {
        let block_hash = value.fuel_block_hash.as_slice();
        let Ok(block_hash) = block_hash.try_into() else {
            bail!("Expected 32 bytes for `fuel_block_hash`, but got: {block_hash:?} from db",);
        };

        let Ok(block_height) = value.fuel_block_height.try_into() else {
            bail!(
                "`fuel_block_height` as read from the db cannot fit in a `u32` as expected. Got: {:?} from db",
                value.fuel_block_height

            );
        };

        let Ok(submittal_height) = value.submittal_height.try_into() else {
            bail!("`submittal_height` as read from the db cannot fit in a `u64` as expected. Got: {} from db", value.submittal_height);
        };

        Ok(Self {
            block_hash,
            block_height,
            completed: value.completed,
            submittal_height,
        })
    }
}

impl From<BlockSubmission> for L1FuelBlockSubmission {
    fn from(value: BlockSubmission) -> Self {
        Self {
            fuel_block_hash: value.block_hash.to_vec(),
            fuel_block_height: i64::from(value.block_height),
            completed: value.completed,
            submittal_height: value.submittal_height.into(),
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct L1StateSubmission {
    pub fuel_block_height: i64,
    pub is_completed: bool,
    pub num_fragments: i64,
}

#[derive(sqlx::FromRow)]
pub struct L1StateFragment {
    pub state_submission: i64,
    pub raw_data: Vec<u8>,
    pub is_completed: bool,
    pub fragment_index: i64,
}

impl TryFrom<L1StateSubmission> for StateSubmission {
    type Error = crate::error::Error;

    fn try_from(value: L1StateSubmission) -> Result<Self, Self::Error> {
        let fuel_block_height = value.fuel_block_height.try_into();
        let Ok(fuel_block_height) = fuel_block_height else {
            bail!(
                "`fuel_block_height` as read from the db cannot fit in a `u32` as expected. Got: {} from db",
                value.fuel_block_height
            );
        };

        let num_fragments = value.num_fragments.try_into();
        let Ok(num_fragments) = num_fragments else {
            bail!(
                "`num_fragments` as read from the db cannot fit in a `u32` as expected. Got: {} from db",
                value.num_fragments
            );
        };

        Ok(Self {
            fuel_block_height,
            is_completed: value.is_completed,
            num_fragments,
        })
    }
}

impl From<StateSubmission> for L1StateSubmission {
    fn from(value: StateSubmission) -> Self {
        Self {
            fuel_block_height: i64::from(value.fuel_block_height),
            is_completed: value.is_completed,
            num_fragments: i64::from(value.num_fragments),
        }
    }
}

impl TryFrom<L1StateFragment> for StateFragment {
    type Error = crate::error::Error;

    fn try_from(value: L1StateFragment) -> Result<Self, Self::Error> {
        let state_submission = value.state_submission.try_into();
        let Ok(state_submission) = state_submission else {
            bail!(
                "`state_submission` as read from the db cannot fit in a `u32` as expected. Got: {} from db",
                value.state_submission
            );
        };

        let fragment_index = value.fragment_index.try_into();
        let Ok(fragment_index) = fragment_index else {
            bail!(
                "`fragment_index` as read from the db cannot fit in a `u32` as expected. Got: {} from db",
                value.fragment_index
            );
        };

        Ok(Self {
            state_submission,
            raw_data: value.raw_data,
            is_completed: value.is_completed,
            fragment_index,
        })
    }
}

impl From<StateFragment> for L1StateFragment {
    fn from(value: StateFragment) -> Self {
        Self {
            state_submission: i64::from(value.state_submission),
            raw_data: value.raw_data,
            is_completed: value.is_completed,
            fragment_index: i64::from(value.fragment_index),
        }
    }
}
