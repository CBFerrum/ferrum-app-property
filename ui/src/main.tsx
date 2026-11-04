// Property's own interface at /property/: the units by building with their leases and requests,
// with every record in the records frame's lists and forms. The same release draws its page of the
// workspace's portal, "Your home" at /portal/home/ (Ferrum docs/plan/apps-ui.md §9.1).

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StrictMode, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { createBrowserRouter, Link, Outlet, RouterProvider, useRouteError } from "react-router";
import { bootstrap, routeBase, shell } from "@ferrum/client";
import { ConfirmHost, EmptyState, ErrorState, Toaster } from "@ferrum/kit";
import { RequireAuth, setFrameMode } from "@ferrum/records/frame";
import "./styles.css";

const page = (name: "FormView" | "ListView" | "ReportView" | "AppSettingsPage" | "ScreenPage") => () => import("./records").then((m) => ({ Component: m[name] }));

const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false, staleTime: 5_000 } } });

function RouteError() {
  return <ErrorState error={useRouteError()} onRetry={() => window.location.reload()} />;
}

function NotFound({ back = "Go to the units" }: { back?: string }) {
  return (
    <EmptyState title="Page not found">
      <Link to="/">{back}</Link>
    </EmptyState>
  );
}

function Started() {
  useEffect(() => shell().ready(), []);
  return null;
}

/** The portal's page, when the platform serves this release there. */
const PORTAL = "portal/home";
const onPortal = bootstrap()?.app?.entry === PORTAL;

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
      { path: "*", element: <NotFound back="Go to your home" /> },
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
      { path: "apps/:app/settings", lazy: page("AppSettingsPage") },
      { path: "x/:app/:page", lazy: page("ScreenPage") },
      { path: "*", element: <NotFound /> },
    ],
  },
];

const router = onPortal
  ? createBrowserRouter(portalRoutes, { basename: routeBase(PORTAL) })
  : createBrowserRouter(staffRoutes, { basename: routeBase("property") });

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      <Toaster />
      <ConfirmHost />
    </QueryClientProvider>
  </StrictMode>,
);
