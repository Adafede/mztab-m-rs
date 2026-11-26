//! Validation layer and severity override configuration.

use alloc::borrow::ToOwned as _;

use mztab_m::validate::Config;
use mztab_m::{Code, Severity};

use super::{Error, Layer};

/// How to validate: the last layer to run and the severity settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// The last layer to run.
    pub layer: Layer,
    /// Severity overrides and disabled codes.
    pub config: Config,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            layer: Layer::Semantic,
            config: Config::default(),
        }
    }
}

impl Settings {
    /// With `layer` as the last layer.
    #[must_use]
    pub const fn layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }

    /// With finding `id` overridden to `level` (`error`, `warning`, `info` or
    /// `off`, not reported).
    ///
    /// # Errors
    ///
    /// If `id` names no code or `level` is not a valid severity.
    pub fn set_code(mut self, id: &str, level: &str) -> Result<Self, Error> {
        let code: Code = id.parse().map_err(|_| Error::Code(id.to_owned()))?;
        self.config = if level == "off" {
            self.config.without(code)
        } else {
            let parsed = level
                .parse::<Severity>()
                .map_err(|_| Error::Severity(level.to_owned()))?;
            self.config.with_severity(code, parsed)
        };
        Ok(self)
    }

    /// With `CODE=LEVEL` applied: LEVEL is `error`, `warning`, `info` or
    /// `off` (not reported).
    ///
    /// ```
    /// use mztab_m::Code;
    /// use mztab_m_api::Settings;
    ///
    /// let settings = Settings::default().set("MZT-X002=off")?;
    /// assert_eq!(settings.config.severity_of(Code::CvUnused), None);
    /// # Ok::<(), mztab_m_api::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// If `setting` is not of that form, or names no code.
    pub fn set(self, setting: &str) -> Result<Self, Error> {
        let (id, level) = setting
            .split_once('=')
            .ok_or_else(|| Error::Setting(setting.to_owned()))?;
        self.set_code(id, level)
    }
}
