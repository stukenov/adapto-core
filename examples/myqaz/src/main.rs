mod data;
mod html;
mod lang;
mod routes;
mod types;

use adapto_app::App;

#[tokio::main]
async fn main() {
    let store = adapto_store::AdaptoStore::open(None).unwrap();
    data::import_all(&store);
    routes::static_pages::import_static_pages(&store);

    let app = App::new("myqaz.kz")
        .port(3002)
        .store(store);

    let app = register_static_routes(app);
    let app = register_routes(app);
    let app = register_kz_routes(app);

    app.run().await.unwrap();
}

fn register_static_routes(app: App) -> App {
    app
        .page("/", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/")
        })
        .page("/kz", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/kz")
        })
        .page("/law", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/law")
        })
        .page("/islam", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/islam")
        })
        .page("/christianity", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/christianity")
        })
        .page("/auto", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/auto")
        })
        .page("/blocked-sites", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/blocked-sites")
        })
        .page("/law/orders", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/law/orders")
        })
        .page("/law/presidential", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/law/presidential")
        })
        .page("/reference", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/reference")
        })
        .page("/reference/budget-codes", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/reference/budget-codes")
        })
        .page("/reference/payment-codes", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/reference/payment-codes")
        })
        .page("/reference/economic-activity", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/reference/economic-activity")
        })
        .page("/directory/government/agencies", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/directory/government/agencies")
        })
        .page("/directory/government/ministries", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/directory/government/ministries")
        })
        .page("/directory/government/central", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/directory/government/central")
        })
        .page("/directory/government/regions", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/directory/government/regions")
        })
        .page("/browse/abroad", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/abroad")
        })
        .page("/browse/benefits", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/benefits")
        })
        .page("/browse/births-marriages-deaths", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/births-marriages-deaths")
        })
        .page("/browse/business", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/business")
        })
        .page("/browse/childcare-parenting", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/childcare-parenting")
        })
        .page("/browse/citizenship", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/citizenship")
        })
        .page("/browse/disability", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/disability")
        })
        .page("/browse/driving", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/driving")
        })
        .page("/browse/education", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/education")
        })
        .page("/browse/employing", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/employing")
        })
        .page("/browse/environment", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/environment")
        })
        .page("/browse/housing", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/housing")
        })
        .page("/browse/justice", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/justice")
        })
        .page("/browse/money-tax", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/money-tax")
        })
        .page("/browse/visas", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/visas")
        })
        .page("/browse/working", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/browse/working")
        })
}

fn register_routes(app: App) -> App {
    app
        // Financial indicators
        .page("/reference/financial-indicators", |ctx| {
            routes::financial::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/reference/financial-indicators/:slug", |ctx| {
            let slug = ctx.param("slug");
            if slug.chars().all(|c| c.is_ascii_digit()) {
                routes::financial::render_year(ctx.store(), slug, lang::Lang::Ru)
            } else {
                routes::financial::render_indicator(ctx.store(), slug, lang::Lang::Ru)
            }
        })
        // Measurement units
        .page("/reference/measurement-units", |ctx| {
            routes::measurement::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/reference/measurement-units/:group", |ctx| {
            routes::measurement::render_group(ctx.store(), ctx.param("group"), lang::Lang::Ru)
        })
        .page("/reference/measurement-units/:group/:code", |ctx| {
            routes::measurement::render_item(ctx.store(), ctx.param("group"), ctx.param("code"), lang::Lang::Ru)
        })
        // Phone codes
        .page("/reference/phone-codes", |ctx| {
            routes::phone_codes::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/reference/phone-codes/:slug", |ctx| {
            routes::phone_codes::render_city(ctx.store(), ctx.param("slug"), lang::Lang::Ru)
        })
        // Postal codes
        .page("/reference/postal-codes", |ctx| {
            routes::postal_codes::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/reference/postal-codes/:region", |ctx| {
            routes::postal_codes::render_region(ctx.store(), ctx.param("region"), lang::Lang::Ru)
        })
        .page("/reference/postal-codes/:region/:index", |ctx| {
            routes::postal_codes::render_code(ctx.store(), ctx.param("region"), ctx.param("index"), lang::Lang::Ru)
        })
        // Waste codes
        .page("/reference/waste-codes", |ctx| {
            routes::waste_codes::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/reference/waste-codes/:group", |ctx| {
            routes::waste_codes::render_group(ctx.store(), ctx.param("group"), lang::Lang::Ru)
        })
        .page("/reference/waste-codes/:group/:subgroup", |ctx| {
            routes::waste_codes::render_subgroup(ctx.store(), ctx.param("group"), ctx.param("subgroup"), lang::Lang::Ru)
        })
        .page("/reference/waste-codes/:group/:subgroup/:item", |ctx| {
            routes::waste_codes::render_item(ctx.store(), ctx.param("group"), ctx.param("subgroup"), ctx.param("item"), lang::Lang::Ru)
        })
        // Tax rates
        .page("/reference/tax-rates", |ctx| {
            routes::tax_rates::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/reference/tax-rates/:slug", |ctx| {
            routes::tax_rates::render_item(ctx.store(), ctx.param("slug"), lang::Lang::Ru)
        })
        .page("/reference/tax-rates/:slug/:sub", |ctx| {
            routes::tax_rates::render_sub_item(ctx.store(), ctx.param("slug"), ctx.param("sub"), lang::Lang::Ru)
        })
        // Notaries
        .page("/directory/notaries", |ctx| {
            routes::notaries::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/directory/notaries/:region", |ctx| {
            routes::notaries::render_region(ctx.store(), ctx.param("region"), lang::Lang::Ru)
        })
        .page("/directory/notaries/:region/:notary", |ctx| {
            routes::notaries::render_notary(ctx.store(), ctx.param("region"), ctx.param("notary"), lang::Lang::Ru)
        })
        // Bailiffs
        .page("/directory/bailiffs", |ctx| {
            routes::bailiffs::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/directory/bailiffs/:region", |ctx| {
            routes::bailiffs::render_region(ctx.store(), ctx.param("region"), lang::Lang::Ru)
        })
        .page("/directory/bailiffs/:region/:bailiff", |ctx| {
            routes::bailiffs::render_bailiff(ctx.store(), ctx.param("region"), ctx.param("bailiff"), lang::Lang::Ru)
        })
        // Namaz
        .page("/islam/namaz", |ctx| {
            routes::namaz::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/islam/namaz/:city", |ctx| {
            routes::namaz::render_city(ctx.store(), ctx.param("city"), lang::Lang::Ru)
        })
        .page("/islam/namaz/:city/:month", |ctx| {
            routes::namaz::render_month(ctx.store(), ctx.param("city"), ctx.param("month"), lang::Lang::Ru)
        })
        .page("/islam/namaz/:city/:month/:day", |ctx| {
            routes::namaz::render_day(ctx.store(), ctx.param("city"), ctx.param("month"), ctx.param("day"), lang::Lang::Ru)
        })
        .page("/islam/namaz/:city/:month/:day/:prayer", |ctx| {
            routes::namaz::render_prayer(ctx.store(), ctx.param("city"), ctx.param("month"), ctx.param("day"), ctx.param("prayer"), lang::Lang::Ru)
        })
        // Quran
        .page("/islam/quran", |ctx| {
            routes::quran::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/islam/quran/:surah", |ctx| {
            routes::quran::render_surah(ctx.store(), ctx.param("surah"), lang::Lang::Ru)
        })
        .page("/islam/quran/:surah/:verse", |ctx| {
            routes::quran::render_ayah(ctx.store(), ctx.param("surah"), ctx.param("verse"), lang::Lang::Ru)
        })
        // Hadith
        .page("/islam/hadith", |ctx| {
            routes::hadith::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/islam/hadith/:category", |ctx| {
            routes::hadith::render_category(ctx.store(), ctx.param("category"), lang::Lang::Ru)
        })
        .page("/islam/hadith/:category/:id", |ctx| {
            routes::hadith::render_hadith(ctx.store(), ctx.param("category"), ctx.param("id"), lang::Lang::Ru)
        })
        // Bible
        .page("/christianity/bible", |ctx| {
            routes::bible::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/christianity/bible/:book", |ctx| {
            routes::bible::render_book(ctx.store(), ctx.param("book"), lang::Lang::Ru)
        })
        .page("/christianity/bible/:book/:chapter", |ctx| {
            routes::bible::render_chapter(ctx.store(), ctx.param("book"), ctx.param("chapter"), lang::Lang::Ru)
        })
        // Legal codes
        .page("/law/codes", |ctx| {
            routes::codes::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/law/codes/:code", |ctx| {
            routes::codes::render_code(ctx.store(), ctx.param("code"), lang::Lang::Ru)
        })
        .page("/law/codes/:code/:chapter", |ctx| {
            routes::codes::render_chapter(ctx.store(), ctx.param("code"), ctx.param("chapter"), lang::Lang::Ru)
        })
        .page("/law/codes/:code/:chapter/:article", |ctx| {
            routes::codes::render_article(ctx.store(), ctx.param("code"), ctx.param("chapter"), ctx.param("article"), lang::Lang::Ru)
        })
        // Laws
        .page("/law/laws", |ctx| {
            routes::laws::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/law/laws/:law", |ctx| {
            routes::laws::render_law(ctx.store(), ctx.param("law"), lang::Lang::Ru)
        })
        .page("/law/laws/:law/:article", |ctx| {
            routes::laws::render_article(ctx.store(), ctx.param("law"), ctx.param("article"), lang::Lang::Ru)
        })
        // Constitution
        .page("/law/constitution", |ctx| {
            routes::constitution::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/law/constitution/:section", |ctx| {
            routes::constitution::render_section(ctx.store(), ctx.param("section"), lang::Lang::Ru)
        })
        .page("/law/constitution/:section/:article", |ctx| {
            routes::constitution::render_article(ctx.store(), ctx.param("section"), ctx.param("article"), lang::Lang::Ru)
        })
        // Classifiers (payment codes)
        .page("/reference/classifiers", |ctx| {
            routes::payment_codes::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/reference/classifiers/:slug", |ctx| {
            routes::payment_codes::render_classifier(ctx.store(), ctx.param("slug"), lang::Lang::Ru)
        })
        // Shezhire
        .page("/shezhire", |ctx| {
            routes::shezhire::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/shezhire/:era", |ctx| {
            routes::shezhire::render_era(ctx.store(), ctx.param("era"), lang::Lang::Ru)
        })
        .page("/shezhire/:era/:tribe", |ctx| {
            routes::shezhire::render_tribe(ctx.store(), ctx.param("era"), ctx.param("tribe"), lang::Lang::Ru)
        })
        // Government orgs (index is static, individual orgs dynamic)
        .page("/directory/government", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/directory/government")
        })
        .page("/directory/government/:slug", |ctx| {
            routes::gov_orgs::render_org(ctx.store(), ctx.param("slug"), lang::Lang::Ru)
        })
        // Decrees
        .page("/law/decrees", |ctx| {
            routes::decrees::render_index(ctx.store(), lang::Lang::Ru)
        })
        // Drugs
        .page("/reference/drugs", |ctx| {
            routes::drugs::render_index(ctx.store(), lang::Lang::Ru)
        })
        .page("/reference/drugs/:id", |ctx| {
            routes::drugs::render_drug(ctx.store(), ctx.param("id"), lang::Lang::Ru)
        })
        // Companies (index is static year/month grid, individual pages dynamic)
        .page("/directory/companies", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/directory/companies")
        })
        .page("/directory/companies/:bin", |ctx| {
            routes::companies::render_company(ctx.store(), ctx.param("bin"), lang::Lang::Ru)
        })
}

fn register_kz_routes(app: App) -> App {
    app
        // Financial indicators
        .page("/kz/reference/financial-indicators", |ctx| {
            routes::financial::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/reference/financial-indicators/:slug", |ctx| {
            let slug = ctx.param("slug");
            if slug.chars().all(|c| c.is_ascii_digit()) {
                routes::financial::render_year(ctx.store(), slug, lang::Lang::Kk)
            } else {
                routes::financial::render_indicator(ctx.store(), slug, lang::Lang::Kk)
            }
        })
        // Measurement units
        .page("/kz/reference/measurement-units", |ctx| {
            routes::measurement::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/reference/measurement-units/:group", |ctx| {
            routes::measurement::render_group(ctx.store(), ctx.param("group"), lang::Lang::Kk)
        })
        .page("/kz/reference/measurement-units/:group/:code", |ctx| {
            routes::measurement::render_item(ctx.store(), ctx.param("group"), ctx.param("code"), lang::Lang::Kk)
        })
        // Phone codes
        .page("/kz/reference/phone-codes", |ctx| {
            routes::phone_codes::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/reference/phone-codes/:slug", |ctx| {
            routes::phone_codes::render_city(ctx.store(), ctx.param("slug"), lang::Lang::Kk)
        })
        // Postal codes
        .page("/kz/reference/postal-codes", |ctx| {
            routes::postal_codes::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/reference/postal-codes/:region", |ctx| {
            routes::postal_codes::render_region(ctx.store(), ctx.param("region"), lang::Lang::Kk)
        })
        .page("/kz/reference/postal-codes/:region/:index", |ctx| {
            routes::postal_codes::render_code(ctx.store(), ctx.param("region"), ctx.param("index"), lang::Lang::Kk)
        })
        // Waste codes
        .page("/kz/reference/waste-codes", |ctx| {
            routes::waste_codes::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/reference/waste-codes/:group", |ctx| {
            routes::waste_codes::render_group(ctx.store(), ctx.param("group"), lang::Lang::Kk)
        })
        .page("/kz/reference/waste-codes/:group/:subgroup", |ctx| {
            routes::waste_codes::render_subgroup(ctx.store(), ctx.param("group"), ctx.param("subgroup"), lang::Lang::Kk)
        })
        .page("/kz/reference/waste-codes/:group/:subgroup/:item", |ctx| {
            routes::waste_codes::render_item(ctx.store(), ctx.param("group"), ctx.param("subgroup"), ctx.param("item"), lang::Lang::Kk)
        })
        // Tax rates
        .page("/kz/reference/tax-rates", |ctx| {
            routes::tax_rates::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/reference/tax-rates/:slug", |ctx| {
            routes::tax_rates::render_item(ctx.store(), ctx.param("slug"), lang::Lang::Kk)
        })
        .page("/kz/reference/tax-rates/:slug/:sub", |ctx| {
            routes::tax_rates::render_sub_item(ctx.store(), ctx.param("slug"), ctx.param("sub"), lang::Lang::Kk)
        })
        // Notaries
        .page("/kz/directory/notaries", |ctx| {
            routes::notaries::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/directory/notaries/:region", |ctx| {
            routes::notaries::render_region(ctx.store(), ctx.param("region"), lang::Lang::Kk)
        })
        .page("/kz/directory/notaries/:region/:notary", |ctx| {
            routes::notaries::render_notary(ctx.store(), ctx.param("region"), ctx.param("notary"), lang::Lang::Kk)
        })
        // Bailiffs
        .page("/kz/directory/bailiffs", |ctx| {
            routes::bailiffs::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/directory/bailiffs/:region", |ctx| {
            routes::bailiffs::render_region(ctx.store(), ctx.param("region"), lang::Lang::Kk)
        })
        .page("/kz/directory/bailiffs/:region/:bailiff", |ctx| {
            routes::bailiffs::render_bailiff(ctx.store(), ctx.param("region"), ctx.param("bailiff"), lang::Lang::Kk)
        })
        // Namaz
        .page("/kz/islam/namaz", |ctx| {
            routes::namaz::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/islam/namaz/:city", |ctx| {
            routes::namaz::render_city(ctx.store(), ctx.param("city"), lang::Lang::Kk)
        })
        .page("/kz/islam/namaz/:city/:month", |ctx| {
            routes::namaz::render_month(ctx.store(), ctx.param("city"), ctx.param("month"), lang::Lang::Kk)
        })
        .page("/kz/islam/namaz/:city/:month/:day", |ctx| {
            routes::namaz::render_day(ctx.store(), ctx.param("city"), ctx.param("month"), ctx.param("day"), lang::Lang::Kk)
        })
        .page("/kz/islam/namaz/:city/:month/:day/:prayer", |ctx| {
            routes::namaz::render_prayer(ctx.store(), ctx.param("city"), ctx.param("month"), ctx.param("day"), ctx.param("prayer"), lang::Lang::Kk)
        })
        // Quran
        .page("/kz/islam/quran", |ctx| {
            routes::quran::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/islam/quran/:surah", |ctx| {
            routes::quran::render_surah(ctx.store(), ctx.param("surah"), lang::Lang::Kk)
        })
        .page("/kz/islam/quran/:surah/:verse", |ctx| {
            routes::quran::render_ayah(ctx.store(), ctx.param("surah"), ctx.param("verse"), lang::Lang::Kk)
        })
        // Hadith
        .page("/kz/islam/hadith", |ctx| {
            routes::hadith::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/islam/hadith/:category", |ctx| {
            routes::hadith::render_category(ctx.store(), ctx.param("category"), lang::Lang::Kk)
        })
        .page("/kz/islam/hadith/:category/:id", |ctx| {
            routes::hadith::render_hadith(ctx.store(), ctx.param("category"), ctx.param("id"), lang::Lang::Kk)
        })
        // Bible
        .page("/kz/christianity/bible", |ctx| {
            routes::bible::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/christianity/bible/:book", |ctx| {
            routes::bible::render_book(ctx.store(), ctx.param("book"), lang::Lang::Kk)
        })
        .page("/kz/christianity/bible/:book/:chapter", |ctx| {
            routes::bible::render_chapter(ctx.store(), ctx.param("book"), ctx.param("chapter"), lang::Lang::Kk)
        })
        // Legal codes
        .page("/kz/law/codes", |ctx| {
            routes::codes::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/law/codes/:code", |ctx| {
            routes::codes::render_code(ctx.store(), ctx.param("code"), lang::Lang::Kk)
        })
        .page("/kz/law/codes/:code/:chapter", |ctx| {
            routes::codes::render_chapter(ctx.store(), ctx.param("code"), ctx.param("chapter"), lang::Lang::Kk)
        })
        .page("/kz/law/codes/:code/:chapter/:article", |ctx| {
            routes::codes::render_article(ctx.store(), ctx.param("code"), ctx.param("chapter"), ctx.param("article"), lang::Lang::Kk)
        })
        // Laws
        .page("/kz/law/laws", |ctx| {
            routes::laws::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/law/laws/:law", |ctx| {
            routes::laws::render_law(ctx.store(), ctx.param("law"), lang::Lang::Kk)
        })
        .page("/kz/law/laws/:law/:article", |ctx| {
            routes::laws::render_article(ctx.store(), ctx.param("law"), ctx.param("article"), lang::Lang::Kk)
        })
        // Constitution
        .page("/kz/law/constitution", |ctx| {
            routes::constitution::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/law/constitution/:section", |ctx| {
            routes::constitution::render_section(ctx.store(), ctx.param("section"), lang::Lang::Kk)
        })
        .page("/kz/law/constitution/:section/:article", |ctx| {
            routes::constitution::render_article(ctx.store(), ctx.param("section"), ctx.param("article"), lang::Lang::Kk)
        })
        // Classifiers
        .page("/kz/reference/classifiers", |ctx| {
            routes::payment_codes::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/reference/classifiers/:slug", |ctx| {
            routes::payment_codes::render_classifier(ctx.store(), ctx.param("slug"), lang::Lang::Kk)
        })
        // Shezhire
        .page("/kz/shezhire", |ctx| {
            routes::shezhire::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/shezhire/:era", |ctx| {
            routes::shezhire::render_era(ctx.store(), ctx.param("era"), lang::Lang::Kk)
        })
        .page("/kz/shezhire/:era/:tribe", |ctx| {
            routes::shezhire::render_tribe(ctx.store(), ctx.param("era"), ctx.param("tribe"), lang::Lang::Kk)
        })
        // Government orgs (index is static)
        .page("/kz/directory/government", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/directory/government")
        })
        .page("/kz/directory/government/:slug", |ctx| {
            routes::gov_orgs::render_org(ctx.store(), ctx.param("slug"), lang::Lang::Kk)
        })
        // Decrees
        .page("/kz/law/decrees", |ctx| {
            routes::decrees::render_index(ctx.store(), lang::Lang::Kk)
        })
        // Drugs
        .page("/kz/reference/drugs", |ctx| {
            routes::drugs::render_index(ctx.store(), lang::Lang::Kk)
        })
        .page("/kz/reference/drugs/:id", |ctx| {
            routes::drugs::render_drug(ctx.store(), ctx.param("id"), lang::Lang::Kk)
        })
        // Companies (index is static)
        .page("/kz/directory/companies", |ctx| {
            routes::static_pages::render_static(ctx.store(), "/directory/companies")
        })
        .page("/kz/directory/companies/:bin", |ctx| {
            routes::companies::render_company(ctx.store(), ctx.param("bin"), lang::Lang::Kk)
        })
}
