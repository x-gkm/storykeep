import { QueryClientProvider, type QueryClient } from '@tanstack/react-query'
import { useState, type ReactNode } from 'react'
import { BrowserRouter, Link, Route, Routes } from 'react-router'
import { createQueryClient } from './api/queries'
import { AuthProvider } from './auth/AuthProvider'
import { RedirectIfAuthenticated, RequireAuth } from './auth/guards'
import { AppLayout, AuthLayout } from './components/Layout'
import { EmptyState } from './components/ui'
import { AccountPage } from './pages/AccountPage'
import { CapsuleDetailPage } from './pages/capsule/CapsuleDetailPage'
import { CapsuleListPage } from './pages/capsule/CapsuleListPage'
import { NewCapsulePage } from './pages/capsule/NewCapsulePage'
import { DashboardPage } from './pages/DashboardPage'
import { LoginPage } from './pages/LoginPage'
import { EditMemoryPage } from './pages/memory/EditMemoryPage'
import { MemoryDetailPage } from './pages/memory/MemoryDetailPage'
import { NewMemoryPage } from './pages/memory/NewMemoryPage'
import { NewProfilePage } from './pages/NewProfilePage'
import { ProfilePage } from './pages/profile/ProfilePage'
import { RegisterPage } from './pages/RegisterPage'
import { MembersPage } from './pages/relationship/MembersPage'
import { RelationshipLayout } from './pages/relationship/RelationshipLayout'
import { SettingsPage } from './pages/relationship/SettingsPage'
import { TimelinePage } from './pages/relationship/TimelinePage'

function NotFound() {
  return (
    <EmptyState title="Page not found">
      <Link to="/">Back to my relationships</Link>
    </EmptyState>
  )
}

export function AppRoutes() {
  return (
    <Routes>
      <Route
        element={
          <RedirectIfAuthenticated>
            <AuthLayout />
          </RedirectIfAuthenticated>
        }
      >
        <Route path="/login" element={<LoginPage />} />
        <Route path="/register" element={<RegisterPage />} />
      </Route>
      <Route
        element={
          <RequireAuth>
            <AppLayout />
          </RequireAuth>
        }
      >
        <Route index element={<DashboardPage />} />
        <Route path="profiles/new" element={<NewProfilePage />} />
        <Route path="profiles/:profileId" element={<ProfilePage />} />
        <Route path="relationships/:relationshipId" element={<RelationshipLayout />}>
          <Route index element={<TimelinePage />} />
          <Route path="memories/new" element={<NewMemoryPage />} />
          <Route path="capsules" element={<CapsuleListPage />} />
          <Route path="capsules/new" element={<NewCapsulePage />} />
          <Route path="members" element={<MembersPage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Route>
        <Route path="memories/:memoryId" element={<MemoryDetailPage />} />
        <Route path="memories/:memoryId/edit" element={<EditMemoryPage />} />
        <Route path="capsules/:capsuleId" element={<CapsuleDetailPage />} />
        <Route path="account" element={<AccountPage />} />
        <Route path="*" element={<NotFound />} />
      </Route>
    </Routes>
  )
}

/** Providers shared by the app and the tests (which pass their own router and client). */
export function AppProviders({ client, children }: { client: QueryClient; children: ReactNode }) {
  return (
    <QueryClientProvider client={client}>
      <AuthProvider>{children}</AuthProvider>
    </QueryClientProvider>
  )
}

export default function App() {
  const [client] = useState(createQueryClient)
  return (
    <BrowserRouter>
      <AppProviders client={client}>
        <AppRoutes />
      </AppProviders>
    </BrowserRouter>
  )
}
