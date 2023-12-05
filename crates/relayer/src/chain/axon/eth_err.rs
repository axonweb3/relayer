use ethers::abi::{Detokenize, ParamType, Uint};

/// Panic(uint)
pub struct Panic(Uint);

impl Panic {
    // Can't get the right selector with `derive(EthError)`, so I implement this manually.
    pub fn decode_with_selector(bytes: &[u8]) -> Option<Self> {
        let bytes = bytes.strip_prefix(b"\x4e\x48\x7b\x71")?;
        let tokens = ethers::abi::decode(&[ParamType::Uint(32)], bytes).ok()?;
        Some(Panic(Uint::from_tokens(tokens).ok()?))
    }
}

impl std::fmt::Display for Panic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let e = PanicError::from_code(self.0.low_u32());
        write!(f, "{e}")
    }
}

#[derive(Copy, Clone)]
enum PanicError {
    Generic,
    AssertFailed,
    ArithmeticOverflow = 0x11,
    DivisionByZero,
    InvalidEnumConversion = 0x21,
    InvalidEncoding,
    EmptyArrayPop = 0x31,
    OutOfBoundsAccess,
    ExcessiveAllocation = 0x41,
    UninitializedInternalFunction = 0x51,
    Unknown,
}

impl PanicError {
    fn from_code(code: u32) -> Self {
        match code {
            0 => PanicError::Generic,
            0x01 => PanicError::AssertFailed,
            0x11 => PanicError::ArithmeticOverflow,
            0x12 => PanicError::DivisionByZero,
            0x21 => PanicError::InvalidEnumConversion,
            0x22 => PanicError::InvalidEncoding,
            0x31 => PanicError::EmptyArrayPop,
            0x32 => PanicError::OutOfBoundsAccess,
            0x41 => PanicError::ExcessiveAllocation,
            0x51 => PanicError::UninitializedInternalFunction,
            _ => PanicError::Unknown,
        }
    }
}

impl std::fmt::Display for PanicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error = *self;
        match error {
            PanicError::Generic => write!(
                f,
                "Panic code: 0x{:x}, Generic compiler inserted panic",
                error as u16
            ),
            PanicError::AssertFailed => {
                write!(f, "Panic code: 0x{:x}, Assertion failed", error as u16)
            }
            PanicError::ArithmeticOverflow => write!(
                f,
                "Panic code: 0x{:x}, Arithmetic operation resulted in overflow",
                error as u16
            ),
            PanicError::DivisionByZero => {
                write!(
                    f,
                    "Panic code: 0x{:x}, Division or modulo by zero",
                    error as u16
                )
            }
            PanicError::InvalidEnumConversion => {
                write!(
                    f,
                    "Panic code: 0x{:x}, Invalid enum conversion",
                    error as u16
                )
            }
            PanicError::InvalidEncoding => {
                write!(f, "Panic code: 0x{:x}, Invalid encoding", error as u16)
            }
            PanicError::EmptyArrayPop => write!(
                f,
                "Panic code: 0x{:x}, Attempted to pop an empty array",
                error as u16
            ),
            PanicError::OutOfBoundsAccess => {
                write!(f, "Panic code: 0x{:x}, Out-of-bounds access", error as u16)
            }
            PanicError::ExcessiveAllocation => write!(
                f,
                "Panic code: 0x{:x}, Excessive memory allocation",
                error as u16
            ),
            PanicError::UninitializedInternalFunction => {
                write!(
                    f,
                    "Panic code: 0x{:x}, Called an uninitialized internal function",
                    error as u16
                )
            }
            PanicError::Unknown => write!(f, "Panic code: 0x{:x}, Unknown panic", error as u16),
        }
    }
}

#[cfg(test)]
mod test {
    use ethers::contract::EthError;

    use super::Panic;

    fn parse_abi_err_data(err: &str) -> String {
        let revert_data = hex::decode(
            err.strip_prefix("Contract call reverted with data: 0x")
                .unwrap(),
        )
        .unwrap();
        if let Some(p) = Panic::decode_with_selector(&revert_data) {
            p.to_string()
        } else if let Some(s) = String::decode_with_selector(&revert_data) {
            s
        } else {
            panic!("failed to decode")
        }
    }

    #[test]
    fn test_sol_revert() {
        let err_string = "Contract call reverted with data: 0x08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000001c74657374206661696c656420746f2063726561746520636c69656e7400000000";
        let err = parse_abi_err_data(err_string);
        assert_eq!(err, "test failed to create client");
    }

    #[test]
    fn test_sol_panic() {
        let err_string = "Contract call reverted with data: 0x4e487b710000000000000000000000000000000000000000000000000000000000000012";
        let err = parse_abi_err_data(err_string);
        assert_eq!(err, "Panic code: 0x12, Division or modulo by zero");
    }
}
