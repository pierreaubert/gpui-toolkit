/// Kernel type for density estimation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KernelType {
    #[default]
    Epanechnikov,
    Gaussian,
    Uniform,
}

impl KernelType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Epanechnikov => "Epanechnikov",
            Self::Gaussian => "Gaussian",
            Self::Uniform => "Uniform",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Epanechnikov => Self::Gaussian,
            Self::Gaussian => Self::Uniform,
            Self::Uniform => Self::Epanechnikov,
        }
    }
}
