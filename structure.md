# Plan of the Project Structure
```
├── public/
│   ├── assets/
│   │   ├── fonts/
│   │   ├── images/
│   │   ├── styles/
├── src/
│   ├── main.rs
│   ├── ui/
│   │   ├── app.rs
│   │   ├── layout/
│   │   │   ├── header.rs
│   │   │   ├── footer.rs
│   │   ├── screens/
│   │   │   ├── home.rs
│   │   │   ├── profile.rs
│   │   │   ├── settings.rs
│   │   │   ├── packs.rs (?)
│   │   │   ├── servers.rs
│   │   │   ├── chat.rs
│   │   │   ├── news.rs
│   │   ├── components/
│   │   │   ├── button.rs (?)
│   │   │   ├── modal.rs (?)
│   │   │   ├── input.rs (?)
│   │   │   ├── dropdown.rs (?)
│   ├── core/
│   │   ├── auth.rs
│   │   ├── creeper.rs (?)
│   │   ├── skins.rs (?)
│   │   ├── packs.rs
│   │   ├── stats.rs
│   │   ├── dream_id.rs (?)
│   │   ├── cloud_storage.rs (?)
│   ├── models/
│   │   ├── user.rs
│   │   ├── pack.rs
│   │   ├── server.rs
│   │   ├── config.rs
│   ├── services/
│   │   ├── cloud.rs (?)
│   │   ├── modrinth.rs
│   │   ├── curseforge.rs
│   │   ├── tlauncher.rs
│   │   ├── multi_mc.rs
│   │   ├── discord.rs
│   │   ├── ai_helper.rs (?)
│   │   ├── news_feed.rs
│   ├── utils/
│   │   ├── logger.rs
│   │   ├── path_utils.rs
│   │   ├── platform.rs
│   │   ├── fs_utils.rs
│   ├── config.rs
```