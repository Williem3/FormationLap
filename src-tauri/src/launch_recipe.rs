use crate::{GameLaunchTarget, LaunchRecipe, LaunchSource, SteamLaunchSelector};
use std::path::Path;

const MAX_STEAM_ARGUMENT_BYTES: usize = 4_096;

pub(crate) fn sanitized_target(recipe: &LaunchRecipe) -> Result<GameLaunchTarget, String> {
    match &recipe.source {
        LaunchSource::Steam { .. } => {
            steam_launch_uri(recipe).map(|uri| GameLaunchTarget::Steam { uri })
        }
        LaunchSource::DirectExecutable { executable_path } => Path::new(executable_path)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .map(|executable_name| GameLaunchTarget::DirectExecutable {
                executable_name: executable_name.to_owned(),
            })
            .ok_or_else(|| "direct executable does not have a valid file name".to_owned()),
    }
}

pub(crate) fn steam_launch_uri(recipe: &LaunchRecipe) -> Result<String, String> {
    let LaunchSource::Steam { app_id, selector } = &recipe.source else {
        return Err("launch source is not Steam".to_owned());
    };
    let selector = selector
        .as_ref()
        .ok_or_else(|| "Steam launch selector is missing".to_owned())?;
    if recipe.arguments.iter().map(String::len).sum::<usize>() > MAX_STEAM_ARGUMENT_BYTES {
        return Err("Steam launch arguments exceed the safe limit".to_owned());
    }

    if recipe.arguments.is_empty() {
        let selector = match selector {
            SteamLaunchSelector::Default => "option0".to_owned(),
            SteamLaunchSelector::OpenVr => "VR".to_owned(),
            SteamLaunchSelector::Oculus => "OTHERVR".to_owned(),
            SteamLaunchSelector::Option { index } => format!("option{index}"),
        };
        return Ok(format!("steam://launch/{app_id}/{selector}"));
    }

    if selector != &SteamLaunchSelector::Default {
        return Err("Steam launch arguments require the explicit default selector".to_owned());
    }
    let arguments = recipe
        .arguments
        .iter()
        .map(|argument| percent_encode(argument))
        .collect::<Vec<_>>()
        .join("%20");
    Ok(format!("steam://run/{app_id}//{arguments}/"))
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::steam_launch_uri;
    use crate::{LaunchRecipe, LaunchSource, SteamLaunchSelector};

    #[test]
    fn steam_uris_select_an_explicit_mode_or_structured_arguments() {
        let mut recipe = LaunchRecipe {
            source: LaunchSource::Steam {
                app_id: 42,
                selector: Some(SteamLaunchSelector::Default),
            },
            ..LaunchRecipe::default()
        };
        assert_eq!(
            steam_launch_uri(&recipe).expect("default selector should render"),
            "steam://launch/42/option0"
        );

        for (selector, expected) in [
            (SteamLaunchSelector::OpenVr, "steam://launch/42/VR"),
            (SteamLaunchSelector::Oculus, "steam://launch/42/OTHERVR"),
            (
                SteamLaunchSelector::Option { index: 3 },
                "steam://launch/42/option3",
            ),
        ] {
            recipe.source = LaunchSource::Steam {
                app_id: 42,
                selector: Some(selector),
            };
            assert_eq!(
                steam_launch_uri(&recipe).expect("explicit selector should render"),
                expected
            );
        }

        recipe.source = LaunchSource::Steam {
            app_id: 42,
            selector: Some(SteamLaunchSelector::Default),
        };
        recipe.arguments = vec!["-novr".to_owned(), "value with spaces".to_owned()];
        assert_eq!(
            steam_launch_uri(&recipe).expect("structured arguments should render"),
            "steam://run/42//-novr%20value%20with%20spaces/"
        );
    }
}
