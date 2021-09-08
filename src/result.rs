use core::result;
use core::fmt;

const MODULE_BITS: u32 = 9;
const DESCRIPTION_BITS: u32 = 13;
const DEFAULT_VALUE: u32 = 0;
const SUCCESS_VALUE: u32 = DEFAULT_VALUE;

const fn pack_value(module: u32, description: u32) -> u32 {
    module | (description << MODULE_BITS)
}

const fn unpack_module(value: u32) -> u32 {
    value & !(!DEFAULT_VALUE << MODULE_BITS)
}

const fn unpack_description(value: u32) -> u32 {
    (value >> MODULE_BITS) & !(!DEFAULT_VALUE << DESCRIPTION_BITS)
}

pub trait ResultBase {
    fn get_module() -> u32;
    fn get_description() -> u32;

    fn get_value() -> u32 {
        pack_value(Self::get_module(), Self::get_description())
    }

    fn make() -> ResultCode {
        ResultCode::new(Self::get_value())
    }

    fn make_err<T>() -> Result<T> {
        Err(Self::make())
    }

    fn matches(rc: ResultCode) -> bool {
        rc.get_value() == Self::get_value()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
#[repr(C)]
pub struct ResultCode {
    value: u32
}

impl ResultCode {
    pub const fn new(value: u32) -> Self {
        Self { value: value }
    }

    pub fn from<T>(r: Result<T>) -> Self {
        match r {
            Ok(_) => ResultSuccess::make(),
            Err(rc) => rc
        }
    }

    pub fn to<T>(&self, t: T) -> Result<T> {
        match self.is_success() {
            true => Ok(t),
            false => Err(*self)
        }
    }
    
    pub const fn is_success(&self) -> bool {
        self.value == SUCCESS_VALUE
    }
    
    pub const fn is_failure(&self) -> bool {
        !self.is_success()
    }
    
    pub const fn get_value(&self) -> u32 {
        self.value
    }
    
    pub const fn get_module(&self) -> u32 {
        unpack_module(self.value)
    }
    
    pub const fn get_description(&self) -> u32 {
        unpack_description(self.value)
    }
}

impl fmt::Debug for ResultCode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        write!(fmt, "{:#X}", self.value)
    }
}

impl fmt::Display for ResultCode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        write!(fmt, "{:0>4}-{:0>4}", 2000 + self.get_module(), self.get_description())
    }
}

macro_rules! result_define {
    ($name:ident: $module:expr, $description:expr) => {
        paste::paste! {
            pub struct [<Result $name>];

            impl $crate::result::ResultBase for [<Result $name>] {
                fn get_module() -> u32 {
                    $module
                }
                
                fn get_description() -> u32 {
                    $description
                }
            }
        }
    };
}

macro_rules! result_define_group {
    ($module:expr => { $( $name:ident: $description:expr ),* }) => {
        $( result_define!($name: $module, $description); )*
    };
}

macro_rules! result_return_if {
    ($cond:expr, $res:ty) => {
        if $cond {
            return Err(<$res>::make());
        }
    };

    ($cond:expr, $res:literal) => {
        if $cond {
            return Err($crate::result::ResultCode::new($res));
        }
    };
}

macro_rules! result_return_unless {
    ($cond:expr, $res:ty) => {
        result_return_if!(!$cond, $res);
    };

    ($cond:expr, $res:literal) => {
        result_return_if!(!$cond, $res);
    };
}

result_define!(Success: 0, 0);

pub type Result<T> = result::Result<T, ResultCode>;

// Results

pub const RESULT_MODULE: u32 = 503;

result_define_group!(RESULT_MODULE => {
    NotSupported: 1,
    InvalidCast: 2
});