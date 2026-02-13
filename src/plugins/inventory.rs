use azalea::prelude::*;

pub async fn handle(_bot: Client, event: Event, _state: ()) -> anyhow::Result<()> {
    if let Event::Tick = event {
        // Logic to check inventory and drop garbage
        // let menu = bot.menu();
        // for item in menu.items() {
        //     if is_garbage(item) {
        //         // drop item
        //     }
        // }
    }
    Ok(())
}
