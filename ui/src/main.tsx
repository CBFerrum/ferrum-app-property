// Property's own interface at /property/: the units by building with their leases and requests,
// with every record in the records frame's lists and forms. The same release draws its pages of the
// workspace's portal, "Your home" at /portal/home/ and "Properties you own" at /portal/owner/
// (Ferrum docs/plan/apps-ui.md §9.1).

import { useEffect } from "react";
import { Link, Outlet, useRouteError } from "react-router";
import { bootstrap, isolate, shell, t } from "@ferrum/client";
import { EmptyState, ErrorState } from "@ferrum/kit";
import { mountApp, RequireAuth, setFrameMode } from "@ferrum/records/frame";
import "./styles.css";

const page = (name: "FormView" | "ListView" | "ReportView") => () => import("./records").then((m) => ({ Component: m[name] }));

function RouteError() {
  return <ErrorState error={useRouteError()} onRetry={() => window.location.reload()} />;
}

function NotFound({ back = t("Go to the units") }: { back?: string }) {
  return (
    <EmptyState title={t("Page not found")}>
      <Link to="/">{isolate(back)}</Link>
    </EmptyState>
  );
}

function Started() {
  useEffect(() => shell().ready(), []);
  return null;
}

/** The portal's pages, when the platform serves this release there. */
const PORTAL = "portal/home";
const OWNER = "portal/owner";
const entry = bootstrap()?.app?.entry;
const onPortal = entry === PORTAL || entry === OWNER;

if (!onPortal) setFrameMode("app", ["property"]);

const portalRoutes = [
  {
    path: "/",
    element: (
      <main>
        <Started />
        <Outlet />
      </main>
    ),
    errorElement: <RouteError />,
    children: [
      { index: true, lazy: () => import("./Portal").then((m) => ({ Component: m.PortalHome })) },
      { path: "new", lazy: () => import("./Portal").then((m) => ({ Component: m.PortalNewRequest })) },
      { path: "*", get element() { return <NotFound back={t("Go to your home")} />; } },
    ],
  },
];

const ownerRoutes = [
  {
    path: "/",
    element: (
      <main>
        <Started />
        <Outlet />
      </main>
    ),
    errorElement: <RouteError />,
    children: [
      { index: true, lazy: () => import("./Portal").then((m) => ({ Component: m.PortalOwner })) },
      { path: "*", get element() { return <NotFound back={t("Go to your properties")} />; } },
    ],
  },
];

const staffRoutes = [
  {
    path: "/",
    element: (
      <>
        <Started />
        <RequireAuth />
      </>
    ),
    errorElement: <RouteError />,
    children: [
      { index: true, lazy: () => import("./Home").then((m) => ({ Component: m.Home })) },
      { path: "e/:entity", lazy: page("ListView") },
      { path: "e/:entity/:id", lazy: page("FormView") },
      { path: "r/:name", lazy: page("ReportView") },
      { path: "*", element: <NotFound /> },
    ],
  },
];

mountApp({
  catalogs: { ar: () => import("./locales/ar.json"), hi: () => import("./locales/hi.json") },
  ...(entry === OWNER ? { route: OWNER, routes: ownerRoutes } : onPortal ? { route: PORTAL, routes: portalRoutes } : { route: "property", routes: staffRoutes }),
});
