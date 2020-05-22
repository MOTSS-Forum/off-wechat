# Off WeChat

A Telegram bot to archive WeChat official account articles using [monolith](https://github.com/Y2Z/monolith) and then show then in a [hugo](https://github.com/gohugoio/hugo/) [site](https://archives.motss.info/) with [RSS](https://archives.motss.info/index.xml).

The metadata is generated in a **very** fragile way. So it may be broken someday.

## Demo

https://archives.motss.info/

## Deploy

- Edit `config.sample.yml`.
- Run in the working directory.
- Send the bot URLs and then it will reply.
- Send `/update` to update the hugo site.

## Credits

- [monolith](https://github.com/Y2Z/monolith)
- [hugo](https://github.com/gohugoio/hugo/)

## License

Unlicense.
